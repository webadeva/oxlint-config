//! React component / hook / class rules: ports of @eslint-react v5
//! (eslint-plugin-react-x, -naming-convention, -rsc) plus the
//! React-Compiler-backed `react-hooks/config` and `react-hooks/gating`.

mod api_rules;
mod class_rules;
mod component_rules;
pub mod helpers;
mod naming;
mod overlap;
mod purity;
mod react_compiler;
mod rsc;

use serde_json::Value;

use crate::rule::Rule;

/// Upstream rules with `schema: []` accept no options.
pub(crate) fn no_options(options: &[Value]) -> Result<(), String> {
    if options.is_empty() {
        Ok(())
    } else {
        Err("this rule takes no options".into())
    }
}

pub fn rules() -> Vec<Box<dyn Rule>> {
    use class_rules::{LifecycleMember, SetStateIn};
    vec![
        Box::new(class_rules::NoAccessStateInSetstate),
        Box::new(LifecycleMember {
            id: "@eslint-react/no-component-will-mount",
            method: "componentWillMount",
            message: "[Deprecated] Use 'UNSAFE_componentWillMount' instead.",
        }),
        Box::new(LifecycleMember {
            id: "@eslint-react/no-component-will-receive-props",
            method: "componentWillReceiveProps",
            message: "[Deprecated] Use 'UNSAFE_componentWillReceiveProps' instead.",
        }),
        Box::new(LifecycleMember {
            id: "@eslint-react/no-component-will-update",
            method: "componentWillUpdate",
            message: "[Deprecated] Use 'UNSAFE_componentWillUpdate' instead.",
        }),
        Box::new(LifecycleMember {
            id: "@eslint-react/no-unsafe-component-will-mount",
            method: "UNSAFE_componentWillMount",
            message: "Do not use 'UNSAFE_componentWillMount'.",
        }),
        Box::new(LifecycleMember {
            id: "@eslint-react/no-unsafe-component-will-receive-props",
            method: "UNSAFE_componentWillReceiveProps",
            message: "Do not use 'UNSAFE_componentWillReceiveProps'.",
        }),
        Box::new(LifecycleMember {
            id: "@eslint-react/no-unsafe-component-will-update",
            method: "UNSAFE_componentWillUpdate",
            message: "Do not use 'UNSAFE_componentWillUpdate'.",
        }),
        Box::new(SetStateIn {
            id: "@eslint-react/no-set-state-in-component-did-mount",
            method: "componentDidMount",
        }),
        Box::new(SetStateIn {
            id: "@eslint-react/no-set-state-in-component-did-update",
            method: "componentDidUpdate",
        }),
        Box::new(SetStateIn {
            id: "@eslint-react/no-set-state-in-component-will-update",
            method: "componentWillUpdate",
        }),
        Box::new(class_rules::NoUnusedClassComponentMembers),
        Box::new(api_rules::NoCreateRef),
        Box::new(api_rules::NoContextProvider),
        Box::new(api_rules::NoForwardRef),
        Box::new(api_rules::NoUseContext),
        Box::new(component_rules::NoNestedComponentDefinitions),
        Box::new(component_rules::NoNestedLazyComponentDeclarations),
        Box::new(component_rules::NoUnnecessaryUsePrefix),
        Box::new(component_rules::UseState),
        Box::new(purity::Purity),
        Box::new(overlap::ErrorBoundaries),
        Box::new(overlap::NoArrayIndexKey),
        Box::new(overlap::NoCloneElement),
        Box::new(overlap::NoDirectMutationState),
        Box::new(overlap::SetStateInRender),
        Box::new(overlap::StaticComponents),
        Box::new(overlap::UnsupportedSyntax),
        Box::new(overlap::UseMemo),
        Box::new(rsc::FunctionDefinition),
        Box::new(naming::ContextName),
        Box::new(naming::IdName),
        Box::new(naming::RefName),
        Box::new(react_compiler::CompilerCategory {
            id: "react-hooks/config",
        }),
        Box::new(react_compiler::CompilerCategory {
            id: "react-hooks/gating",
        }),
    ]
}
