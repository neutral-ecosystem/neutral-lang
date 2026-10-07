// SPDX-License-Identifier: Apache-2.0

//! Fallible owned identities and exact coefficients used by independently bounded successor paths.

use super::composition::{
    CompositionBindingReference,
    project::{CompositionDeclaration, CompositionSignature},
};
use super::{ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity, VocabularyIdentity};
use super::{
    composition::{
        CompositionAlternative, CompositionBinding, CompositionBody, CompositionBundle,
        CompositionDefinition, CompositionDependency, CompositionField, CompositionValue,
        FieldRestrictions, SourceCompositionDefinition, ValuePathSegment,
    },
    project_interface::ProjectPublicType as T,
};
use neutral_core::allocation::{AllocationError as E, TryClone};

impl TryClone for CompositionSignature {
    /// Retains a bounded complete signature without an infallible wrapper/body copy.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(match self {
            Self::Binding(t) => Self::Binding(t.try_clone()?),
            Self::Definition(b) => Self::Definition(b.try_clone()?),
        })
    }
}
impl TryClone for CompositionDeclaration {
    /// Retains a preflighted declaration atomically, including its materialized value.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            identity: self.identity.try_clone()?,
            public: self.public,
            signature: self.signature.try_clone()?,
            value: self.value.try_clone()?,
        })
    }
}
impl TryClone for CompositionBindingReference {
    /// Retains an exposed reference path and exact target without following that target.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            path: self.path.try_clone()?,
            target: self.target.try_clone()?,
        })
    }
}

impl TryClone for LogicalModuleIdentity {
    /// Copies both logical owner components without an infallible string clone.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            language_behavior_version: self.language_behavior_version.try_clone()?,
            module_name: self.module_name.try_clone()?,
        })
    }
}
impl TryClone for ModuleSymbolIdentity {
    /// Copies the complete stable module-symbol owner fallibly.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            module: self.module.try_clone()?,
            declaration_name: self.declaration_name.try_clone()?,
        })
    }
}
impl TryClone for ExactNumber {
    /// Copies normalized coefficient bytes; signs and scales remain unchanged.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            negative: self.negative,
            coefficient: self.coefficient.try_clone()?,
            scale: self.scale,
        })
    }
}
impl TryClone for VocabularyIdentity {
    /// Copies complete lock facts including features without changing any digest or revision.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            identity: self.identity.try_clone()?,
            version: self.version.try_clone()?,
            schema_version: self.schema_version.try_clone()?,
            encoding_version: self.encoding_version.try_clone()?,
            content_digest: self.content_digest,
            required_features: self.required_features.try_clone()?,
        })
    }
}

impl TryClone for T {
    /// Copies a preflighted type including each recursive wrapper and exact nominal owner.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(match self {
            Self::Num => Self::Num,
            Self::String => Self::String,
            Self::Bool => Self::Bool,
            Self::Url => Self::Url,
            Self::Path => Self::Path,
            Self::Nominal(owner) => Self::Nominal(owner.try_clone()?),
            Self::VocabularyNominal {
                identity,
                version,
                name,
            } => Self::VocabularyNominal {
                identity: identity.try_clone()?,
                version: version.try_clone()?,
                name: name.try_clone()?,
            },
            Self::List(t) => Self::List(t.try_clone()?),
            Self::Ref(t) => Self::Ref(t.try_clone()?),
            Self::Nullable(t) => Self::Nullable(t.try_clone()?),
        })
    }
}
impl<R: TryClone> TryClone for CompositionValue<R> {
    /// Copies a preflighted value with fallible containers, field strings and variant boxes.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(match self {
            Self::Number(n) => Self::Number(n.try_clone()?),
            Self::String(s) => Self::String(s.try_clone()?),
            Self::Url(s) => Self::Url(s.try_clone()?),
            Self::Path(s) => Self::Path(s.try_clone()?),
            Self::Bool(b) => Self::Bool(*b),
            Self::Null => Self::Null,
            Self::Reference(r) => Self::Reference(r.try_clone()?),
            Self::List(v) => Self::List(v.try_clone()?),
            Self::Record(v) => Self::Record(v.try_clone()?),
            Self::Variant { tag, payload } => Self::Variant {
                tag: tag.try_clone()?,
                payload: payload.try_clone()?,
            },
        })
    }
}
impl TryClone for ValuePathSegment {
    /// Copies field keys fallibly without allocating for element or payload selectors.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(match self {
            Self::Field(s) => Self::Field(s.try_clone()?),
            Self::Element(n) => Self::Element(*n),
            Self::Payload => Self::Payload,
        })
    }
}
impl TryClone for FieldRestrictions {
    /// Copies all semantic restrictions without infallible choice/coefficient clones.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            choices: self.choices.try_clone()?,
            minimum: self.minimum.try_clone()?,
            maximum: self.maximum.try_clone()?,
            min_length: self.min_length,
            max_length: self.max_length,
        })
    }
}
impl TryClone for CompositionField {
    /// Copies a previously bounded complete field contract, including unused defaults.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            name: self.name.try_clone()?,
            ty: self.ty.try_clone()?,
            presence: self.presence,
            restrictions: self.restrictions.try_clone()?,
            default: self.default.try_clone()?,
        })
    }
}
impl TryClone for CompositionAlternative {
    /// Copies an alternative tag and its complete payload type.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            tag: self.tag.try_clone()?,
            ty: self.ty.try_clone()?,
        })
    }
}
impl TryClone for CompositionBody {
    /// Copies each preflighted nominal contract kind with fallible recursive ownership.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(match self {
            Self::Record(fs) => Self::Record(fs.try_clone()?),
            Self::Variant(ts) => Self::Variant(ts.try_clone()?),
        })
    }
}
impl TryClone for CompositionDefinition {
    /// Copies a complete bounded definition without discarding visibility or dormant contracts.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            name: self.name.try_clone()?,
            public: self.public,
            body: self.body.try_clone()?,
        })
    }
}
impl TryClone for CompositionDependency {
    /// Copies an exact transitive dependency, never resolving or acquiring it.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            identity: self.identity.try_clone()?,
            version: self.version.try_clone()?,
        })
    }
}
impl TryClone for CompositionBundle {
    /// Copies a preflighted complete bundle using fallible allocations for every owned field.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            identity: self.identity.try_clone()?,
            dependencies: self.dependencies.try_clone()?,
            definitions: self.definitions.try_clone()?,
        })
    }
}

impl TryClone for SourceCompositionDefinition {
    /// Copies both source ownership and the previously bounded complete contract fallibly.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            owner: self.owner.try_clone()?,
            definition: self.definition.try_clone()?,
        })
    }
}
impl TryClone for CompositionBinding {
    /// Copies a bounded resolved binding without infallible owner, type or value retention.
    fn try_clone(&self) -> Result<Self, E> {
        Ok(Self {
            owner: self.owner.try_clone()?,
            public: self.public,
            ty: self.ty.try_clone()?,
            value: self.value.try_clone()?,
        })
    }
}
