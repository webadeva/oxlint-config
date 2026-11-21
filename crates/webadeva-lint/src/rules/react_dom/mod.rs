//! `@eslint-react/*` rules for JSX, `react-dom` and web APIs (ports of
//! eslint-plugin-react-jsx, eslint-plugin-react-dom, eslint-plugin-react-web-api
//! and the list/children rules of eslint-plugin-react-x, v5).

mod dom;
pub mod helpers;
mod jsx;
mod unknown_property;
mod unknown_property_tables;
mod web_api;

use crate::rule::Rule;

pub fn rules() -> Vec<Box<dyn Rule>> {
    let mut rules: Vec<Box<dyn Rule>> = vec![
        Box::new(jsx::JsxNoChildrenProp),
        Box::new(jsx::JsxNoChildrenPropWithChildren),
        Box::new(jsx::JsxNoKeyAfterSpread),
        Box::new(jsx::JsxNoLeakedDollar),
        Box::new(jsx::JsxNoLeakedSemicolon),
        Box::new(jsx::JsxNoNamespace),
        Box::new(jsx::NoMissingKey),
        Box::new(dom::NoDangerouslySetInnerhtml),
        Box::new(dom::NoDangerouslySetInnerhtmlWithChildren),
        Box::new(dom::NoScriptUrl),
        Box::new(dom::NoUnsafeIframeSandbox),
        Box::new(dom::NoVoidElementsWithChildren),
        Box::new(unknown_property::NoUnknownProperty),
    ];
    rules.extend(jsx::children_rules());
    rules.extend(dom::api_call_rules());
    rules.extend(web_api::rules());
    rules
}
