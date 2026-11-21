//! Helpers shared by rules.

use oxc_ast::ast::{JSXElementName, JSXMemberExpressionObject};

/// Name of a JSX element as written (`div`, `Foo`, `Foo.Bar`, `svg:path`).
pub fn jsx_element_name(name: &JSXElementName<'_>) -> String {
    match name {
        JSXElementName::Identifier(id) => id.name.to_string(),
        JSXElementName::IdentifierReference(id) => id.name.to_string(),
        JSXElementName::NamespacedName(ns) => format!("{}:{}", ns.namespace.name, ns.name.name),
        JSXElementName::MemberExpression(m) => {
            let mut parts = vec![m.property.name.to_string()];
            let mut obj = &m.object;
            loop {
                match obj {
                    JSXMemberExpressionObject::IdentifierReference(id) => {
                        parts.push(id.name.to_string());
                        break;
                    }
                    JSXMemberExpressionObject::MemberExpression(inner) => {
                        parts.push(inner.property.name.to_string());
                        obj = &inner.object;
                    }
                    JSXMemberExpressionObject::ThisExpression(_) => {
                        parts.push("this".into());
                        break;
                    }
                }
            }
            parts.reverse();
            parts.join(".")
        }
        JSXElementName::ThisExpression(_) => "this".into(),
    }
}
