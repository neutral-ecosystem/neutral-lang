// SPDX-License-Identifier: Apache-2.0

//! Request-local pure-operation work and fallible scratch retention, shared by the evaluator.

use neutral_core::{CancellationToken, allocation::RetainCapacity};
use neutral_ir::{
    ExactNumber,
    expression::profile::{DECIMAL_SIGNIFICANT_DIGITS, ExpressionLimits},
};

/// Closed safe failures; operands and host details never enter diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OperationError {
    /// An evaluated numeric denominator is zero.
    DivisionByZero,
    /// Independent work, numerical growth, retention or allocation control exhausted.
    Limit,
    /// Cooperative cancellation before publication.
    Cancelled,
    /// An operation was dispatched with an incompatible value kind.
    TypeMismatch,
}

/// Aggregate counters for one evaluation, never reset for an individual operation.
pub(crate) struct Budget<'a> {
    /// Independently intersected positive policy.
    pub(crate) limits: ExpressionLimits,
    /// Explicit cooperative signal.
    cancel: &'a CancellationToken,
    /// Remaining traversal and digit work.
    work: u64,
    /// Cumulative reserved scratch/result bytes, conservative over peak live storage.
    bytes: u64,
}

impl<'a> Budget<'a> {
    /// Copies an already validated residual after aggregate owned-retention preflight.
    pub(crate) fn copy_expression(
        &mut self,
        value: &neutral_ir::expression::Expression,
    ) -> Result<neutral_ir::expression::Expression, OperationError> {
        let limits = self.input_limits();
        let cost = neutral_ir::expression::inspect_expression_retention(value, limits, self.cancel)
            .map_err(shape_error)?;
        self.step(cost.work.checked_mul(2).ok_or(OperationError::Limit)?)?;
        self.storage(cost.retained_bytes)?;
        let result = value
            .try_clone_bounded(limits, self.cancel)
            .map_err(shape_error)?;
        self.step(0)?;
        Ok(result)
    }
    /// Copies an exact annotation without resetting caller depth/work/storage controls.
    pub(crate) fn copy_type(
        &mut self,
        value: &neutral_ir::project_interface::ProjectPublicType,
    ) -> Result<neutral_ir::project_interface::ProjectPublicType, OperationError> {
        use neutral_core::allocation::TryClone;
        let cost = neutral_ir::expression::inspect_expression_type_retention(
            value,
            self.input_limits(),
            self.cancel,
        )
        .map_err(shape_error)?;
        self.step(cost.work)?;
        self.storage(cost.retained_bytes)?;
        let result = value.try_clone();
        self.step(0)?;
        result.map_err(|_| OperationError::Limit)
    }
    /// Returns current controls for nested borrowed preflight; does not reset this budget.
    pub(crate) fn remaining(&self) -> ExpressionLimits {
        ExpressionLimits {
            work: self.work,
            retained_bytes: self.bytes,
            ..self.limits
        }
    }
    /// Input/scratch retention is independent of the final output-byte ceiling.
    pub(crate) fn input_limits(&self) -> ExpressionLimits {
        ExpressionLimits {
            output_bytes: self.bytes,
            ..self.remaining()
        }
    }
    /// Copies one bounded identifier after paying storage and observing final cancellation.
    pub(crate) fn copy_text(&mut self, value: &str) -> Result<String, OperationError> {
        self.storage(value.len() as u64)?;
        let result = neutral_core::allocation::text(value);
        self.step(0)?;
        result.map_err(|_| OperationError::Limit)
    }
    /// Retains a moved value in fallible boxed storage without an unchecked allocation.
    pub(crate) fn boxed<T>(&mut self, value: T) -> Result<Box<T>, OperationError> {
        self.storage(std::mem::size_of::<T>() as u64)?;
        let result = neutral_core::allocation::boxed(value);
        self.step(0)?;
        result.map_err(|_| OperationError::Limit)
    }
    /// Borrows the explicit signal; callers cannot obtain ambient cancellation state.
    pub(crate) fn cancellation(&self) -> &'a CancellationToken {
        self.cancel
    }
    /// Checks output storage before construction or final publication.
    pub(crate) fn output(&mut self, bytes: u64) -> Result<(), OperationError> {
        self.step(0)?;
        if bytes > self.limits.output_bytes {
            return Err(OperationError::Limit);
        }
        Ok(())
    }
    /// Reuses IR's cancellable whole-value copy rather than an allocating recursive clone.
    pub(crate) fn copy_value(
        &mut self,
        value: &neutral_ir::composition::BindingValue,
    ) -> Result<neutral_ir::composition::BindingValue, OperationError> {
        let limits = self.input_limits();
        let cost =
            neutral_ir::expression::inspect_expression_value_retention(value, limits, self.cancel)
                .map_err(shape_error)?;
        self.step(cost.work.checked_mul(2).ok_or(OperationError::Limit)?)?;
        self.storage(cost.retained_bytes)?;
        let result = neutral_ir::expression::try_clone_value_bounded(value, limits, self.cancel)
            .map_err(shape_error)?;
        self.step(0)?;
        Ok(result)
    }
    /// Starts a checked request without acquiring any input or allocating storage.
    pub(crate) fn new(
        limits: ExpressionLimits,
        cancel: &'a CancellationToken,
    ) -> Result<Self, OperationError> {
        let limits = limits.intersect(ExpressionLimits::hard());
        if cancel.is_cancelled() {
            return Err(OperationError::Cancelled);
        }
        if limits.values().contains(&0) {
            return Err(OperationError::Limit);
        }
        Ok(Self {
            limits,
            cancel,
            work: limits.work,
            bytes: limits.retained_bytes,
        })
    }
    /// Charges before proportional traversal and observes cancellation.
    pub(crate) fn step(&mut self, work: u64) -> Result<(), OperationError> {
        if self.cancel.is_cancelled() {
            return Err(OperationError::Cancelled);
        }
        self.work = self.work.checked_sub(work).ok_or(OperationError::Limit)?;
        Ok(())
    }
    /// Bounds a normalized input before coefficient copying or scale alignment.
    pub(crate) fn number(&mut self, value: &ExactNumber) -> Result<(), OperationError> {
        self.step(value.coefficient().len() as u64 + 1)?;
        if value.coefficient().len() as u64 > self.limits.coefficient_digits
            || value.scale().unsigned_abs() > self.limits.absolute_scale
        {
            return Err(OperationError::Limit);
        }
        Ok(())
    }
    /// Bounds algorithmic alignment/carry buffers separately from normalized result digits.
    pub(crate) fn scratch_limit(&self) -> Result<usize, OperationError> {
        let size = self
            .limits
            .coefficient_digits
            .checked_mul(2)
            .and_then(|n| n.checked_add(self.limits.absolute_scale.checked_mul(2)?))
            .and_then(|n| n.checked_add(DECIMAL_SIGNIFICANT_DIGITS as u64 + 2))
            .ok_or(OperationError::Limit)?;
        usize::try_from(size).map_err(|_| OperationError::Limit)
    }
    /// Debits retained bytes before a fallible reservation or owned coefficient copy.
    pub(crate) fn storage(&mut self, bytes: u64) -> Result<(), OperationError> {
        self.step(bytes.checked_add(1).ok_or(OperationError::Limit)?)?;
        self.bytes = self.bytes.checked_sub(bytes).ok_or(OperationError::Limit)?;
        Ok(())
    }
    /// Reserves only absent vector capacity; cancellation wins over an injected failure.
    pub(crate) fn reserve<T>(
        &mut self,
        values: &mut Vec<T>,
        additional: usize,
    ) -> Result<(), OperationError> {
        self.step(0)?;
        let required = values
            .len()
            .checked_add(additional)
            .ok_or(OperationError::Limit)?;
        if required <= values.capacity() {
            return Ok(());
        }
        let bytes = (required - values.capacity())
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(OperationError::Limit)?;
        self.storage(bytes as u64)?;
        let result = values.try_retain_exact(additional);
        self.step(0)?;
        result.map_err(|_| OperationError::Limit)
    }
    /// Constructs initialized bounded digit storage without infallible vector allocation.
    pub(crate) fn zeros(&mut self, length: usize) -> Result<Vec<u8>, OperationError> {
        if length > self.scratch_limit()? {
            return Err(OperationError::Limit);
        }
        let mut value = Vec::new();
        self.reserve(&mut value, length)?;
        value.resize(length, 0);
        Ok(value)
    }
    /// Appends with bounded geometric growth, paying capacity before allocation.
    pub(crate) fn push(&mut self, value: &mut Vec<u8>, digit: u8) -> Result<(), OperationError> {
        self.step(1)?;
        let required = value.len().checked_add(1).ok_or(OperationError::Limit)?;
        let ceiling = self.scratch_limit()?;
        if required > ceiling {
            return Err(OperationError::Limit);
        }
        if required > value.capacity() {
            let capacity = value
                .capacity()
                .saturating_mul(2)
                .max(required)
                .min(ceiling);
            self.reserve(value, capacity - value.len())?;
        }
        value.push(digit);
        Ok(())
    }
}

/// Preserves bounded preflight failure classes without retaining input text.
pub(crate) fn shape_error(error: neutral_ir::expression::ExpressionShapeError) -> OperationError {
    match error {
        neutral_ir::expression::ExpressionShapeError::Cancelled => OperationError::Cancelled,
        neutral_ir::expression::ExpressionShapeError::Limit => OperationError::Limit,
        neutral_ir::expression::ExpressionShapeError::Malformed => OperationError::TypeMismatch,
    }
}
