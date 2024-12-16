use lsp_types::Url;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct BackendConfig {
    #[serde(default)]
    pub(crate) element: Vec<ElementConfig>,
    #[serde(default)]
    pub(crate) component: Vec<ComponentConfig>,
    #[serde(default)]
    pub(crate) global_attribute: Vec<AttributeConfig>,
    #[serde(default)]
    pub(crate) global_event: Vec<EventConfig>,
    #[serde(default)]
    pub(crate) media_type: Vec<MediaTypeConfig>,
    #[serde(default)]
    pub(crate) media_feature: Vec<MediaFeatureConfig>,
    #[serde(default)]
    pub(crate) pseudo_class: Vec<PseudoClassConfig>,
    #[serde(default)]
    pub(crate) pseudo_element: Vec<PseudoElementConfig>,
    #[serde(default)]
    pub(crate) style_property: Vec<StylePropertyConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ElementConfig {
    pub(crate) tag_name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
    #[serde(default)]
    pub(crate) attribute: Vec<AttributeConfig>,
    #[serde(default)]
    pub(crate) event: Vec<EventConfig>,
    #[serde(default)]
    pub(crate) deprecated: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct AttributeConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
    #[serde(default)]
    pub(crate) value_option: Vec<ValueOption>,
    #[serde(default)]
    pub(crate) deprecated: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ComponentConfig {
    pub(crate) tag_name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
    #[serde(default)]
    pub(crate) property: Vec<PropertyConfig>,
    #[serde(default)]
    pub(crate) event: Vec<EventConfig>,
    #[serde(default)]
    pub(crate) deprecated: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PropertyConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) ty: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
    #[serde(default)]
    pub(crate) value_option: Vec<ValueOption>,
    #[serde(default)]
    pub(crate) deprecated: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct ValueOption {
    pub(crate) value: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) deprecated: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct EventConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
    #[serde(default)]
    pub(crate) deprecated: bool,
}

impl BackendConfig {
    pub(crate) fn search_element(&self, tag_name: &str) -> Option<&ElementConfig> {
        self.element.iter().find(|x| x.tag_name == tag_name)
    }

    pub(crate) fn search_component(&self, tag_name: &str) -> Option<&ComponentConfig> {
        self.component.iter().find(|x| x.tag_name == tag_name)
    }

    pub(crate) fn search_attribute(&self, tag_name: &str, attr_name: &str) -> Option<&AttributeConfig> {
        let elem = self.search_element(tag_name)?;
        elem.attribute.iter().chain(self.global_attribute.iter()).find(|x| x.name == attr_name)
    }

    pub(crate) fn search_property(&self, tag_name: &str, attr_name: &str) -> Option<&PropertyConfig> {
        let comp = self.search_component(tag_name)?;
        comp.property.iter().find(|x| x.name == attr_name)
    }

    pub(crate) fn list_attributes(&self, tag_name: &str) -> Option<impl Iterator<Item = &AttributeConfig>> {
        let elem = self.search_element(tag_name)?;
        Some(elem.attribute.iter().chain(self.global_attribute.iter()))
    }

    pub(crate) fn list_properties(&self, tag_name: &str) -> Option<impl Iterator<Item = &PropertyConfig>> {
        let comp = self.search_component(tag_name)?;
        Some(comp.property.iter())
    }

    pub(crate) fn search_global_event(&self, event_name: &str) -> Option<&EventConfig> {
        self.global_event.iter().find(|x| x.name == event_name)
    }

    pub(crate) fn search_element_event(&self, tag_name: &str, event_name: &str) -> Option<&EventConfig> {
        let elem = self.search_element(tag_name)?;
        elem.event.iter().chain(self.global_event.iter()).find(|x| x.name == event_name)
    }

    pub(crate) fn search_component_event(&self, tag_name: &str, event_name: &str) -> Option<&EventConfig> {
        let comp = self.search_component(tag_name)?;
        comp.event.iter().chain(self.global_event.iter()).find(|x| x.name == event_name)
    }

    pub(crate) fn search_event(&self, tag_name: &str, event_name: &str) -> Option<&EventConfig> {
        if self.search_component(tag_name).is_some() {
            self.search_component_event(tag_name, event_name)
        } else {
            self.search_element_event(tag_name, event_name)
        }
    }

    pub(crate) fn list_global_events(&self) -> impl Iterator<Item = &EventConfig> {
        self.global_event.iter()
    }

    pub(crate) fn list_events(&self, tag_name: &str) -> Option<impl Iterator<Item = &EventConfig>> {
        if let Some(comp) = self.search_component(tag_name) {
            Some(comp.event.iter().chain(self.global_event.iter()))
        } else if let Some(elem) = self.search_element(tag_name) {
            Some(elem.event.iter().chain(self.global_event.iter()))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct MediaTypeConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct MediaFeatureConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) ty: Option<MediaFeatureType>,
    #[serde(default)]
    pub(crate) options: Vec<String>,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MediaFeatureType {
    Any,
    Range,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PseudoClassConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct PseudoElementConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct StylePropertyConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
}
