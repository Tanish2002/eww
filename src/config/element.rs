use super::*;

use crate::value::AttrValue;
use hocon_ext::HoconExt;
use maplit::hashmap;
use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(Debug, Clone, PartialEq)]
pub struct WidgetDefinition {
    pub name: String,
    pub structure: WidgetUse,
    pub size: Option<(i32, i32)>,
}

impl WidgetDefinition {
    pub fn parse_hocon(name: String, hocon: &Hocon) -> Result<Self> {
        let definition = hocon.as_hash()?;
        let structure = definition
            .get("structure")
            .cloned()
            .context("structure must be set in widget definition")
            .and_then(WidgetUse::parse_hocon)?;

        Ok(WidgetDefinition {
            name,
            structure,
            size: try {
                (
                    definition.get("size_x")?.as_i64()? as i32,
                    definition.get("size_y")?.as_i64()? as i32,
                )
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WidgetUse {
    pub name: String,
    pub children: Vec<WidgetUse>,
    pub attrs: HashMap<String, AttrValue>,
}

impl WidgetUse {
    pub fn new(name: String, children: Vec<WidgetUse>) -> Self {
        WidgetUse {
            name,
            children,
            attrs: HashMap::new(),
        }
    }

    pub fn parse_hocon(data: Hocon) -> Result<Self> {
        match data {
            Hocon::Hash(data) => {
                let (widget_name, widget_config) = data.into_iter().next().context("tried to parse empty hash as widget use")?;
                match widget_config {
                    Hocon::Hash(widget_config) => WidgetUse::from_hash_definition(widget_name.clone(), widget_config),
                    direct_childen => Ok(WidgetUse::new(
                        widget_name.clone(),
                        parse_widget_use_children(direct_childen)?,
                    )),
                }
            }
            primitive => Ok(WidgetUse::simple_text(AttrValue::try_from(&primitive)?)),
        }
    }

    /// generate a WidgetUse from an array-style definition
    /// i.e.: { layout: [ "hi", "ho" ] }
    pub fn from_array_definition(widget_name: String, children: Vec<Hocon>) -> Result<Self> {
        let children = children.into_iter().map(WidgetUse::parse_hocon).collect::<Result<_>>()?;
        Ok(WidgetUse::new(widget_name, children))
    }

    /// generate a WidgetUse from a hash-style definition
    /// i.e.: { layout: { orientation: "v", children: ["hi", "Ho"] } }
    pub fn from_hash_definition(widget_name: String, mut widget_config: HashMap<String, Hocon>) -> Result<Self> {
        let children = widget_config
            .remove("children")
            .map(parse_widget_use_children)
            .unwrap_or(Ok(Vec::new()))?;

        let attrs = widget_config
            .into_iter()
            .filter_map(|(key, value)| Some((key.to_lowercase(), AttrValue::try_from(&value).ok()?)))
            .collect();

        Ok(WidgetUse {
            name: widget_name.to_string(),
            children,
            attrs,
        })
    }

    pub fn simple_text(text: AttrValue) -> Self {
        WidgetUse {
            name: "label".to_owned(),
            children: vec![],
            attrs: hashmap! { "text".to_string() => text }, // TODO this hardcoded "text" is dumdum
        }
    }

    pub fn get_attr(&self, key: &str) -> Result<&AttrValue> {
        self.attrs
            .get(key)
            .context(format!("attribute '{}' missing from widgetuse of '{}'", key, &self.name))
    }
}

pub fn parse_widget_use_children(children: Hocon) -> Result<Vec<WidgetUse>> {
    match children {
        Hocon::Hash(_) => bail!(
            "children of a widget must either be a list of widgets or a primitive value, but got hash: {:?}",
            children
        ),
        Hocon::Array(widget_children) => widget_children
            .into_iter()
            .map(WidgetUse::parse_hocon)
            .collect::<Result<Vec<_>>>(),
        primitive => Ok(vec![WidgetUse::simple_text(AttrValue::try_from(&primitive)?)]),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use maplit::hashmap;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_widget_use() {
        let input_complex = r#"{
            widget_name: {
                value: "test"
                children: [
                    { child: {} }
                    { child: { children: ["hi"] } }
                ]
            }
        }"#;
        let expected = WidgetUse {
            name: "widget_name".to_string(),
            children: vec![
                WidgetUse::new("child".to_string(), vec![]),
                WidgetUse::new(
                    "child".to_string(),
                    vec![WidgetUse::simple_text(AttrValue::Concrete(PrimitiveValue::String(
                        "hi".to_string(),
                    )))],
                ),
            ],
            attrs: hashmap! { "value".to_string() => AttrValue::Concrete(PrimitiveValue::String("test".to_string()))},
        };
        assert_eq!(
            WidgetUse::parse_hocon(parse_hocon(input_complex).unwrap().clone()).unwrap(),
            expected
        );
    }

    #[test]
    fn test_parse_widget_definition() {
        let input_complex = r#"{
            structure: { foo: {} }
        }"#;
        let expected = WidgetDefinition {
            name: "widget_name".to_string(),
            structure: WidgetUse::new("foo".to_string(), vec![]),
            size: None,
        };
        assert_eq!(
            WidgetDefinition::parse_hocon("widget_name".to_string(), &parse_hocon(input_complex).unwrap()).unwrap(),
            expected
        );
    }
}
