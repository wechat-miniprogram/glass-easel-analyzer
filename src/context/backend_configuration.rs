use std::fmt::Write;

use lsp_types::Url;

use crate::utils::dash_to_camel;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct BackendConfig {
    #[serde(default)]
    pub(crate) glass_easel_backend_config: GlassEaselBackendConfig,
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

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct GlassEaselBackendConfig {
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) major_version: u32,
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) minor_version: u32,
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
pub(crate) struct ComponentConfig {
    pub(crate) tag_name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
    #[serde(default)]
    pub(crate) property: Vec<AttributeConfig>,
    #[serde(default)]
    pub(crate) event: Vec<EventConfig>,
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
    pub(crate) fn parse_str(s: &str) -> anyhow::Result<Self> {
        let config: Self = toml::from_str(s)?;
        let major_version = config.glass_easel_backend_config.major_version;
        if major_version < 1 {
            log::warn!(
                "This backend configuration may be problematic. Please check the updates of it."
            );
        } else if major_version > 1 {
            Err(anyhow::Error::msg("The backend configuration is designed for a later version of glass-easel-analyzer."))?;
        }
        log::info!(
            "Loaded backend configuration: {}",
            config.glass_easel_backend_config.name
        );
        Ok(config)
    }

    pub(crate) fn search_element(&self, tag_name: &str) -> Option<&ElementConfig> {
        self.element.iter().find(|x| x.tag_name == tag_name)
    }

    pub(crate) fn search_component(&self, tag_name: &str) -> Option<&ComponentConfig> {
        self.component.iter().find(|x| x.tag_name == tag_name)
    }

    pub(crate) fn search_attribute(
        &self,
        tag_name: &str,
        attr_name: &str,
    ) -> Option<&AttributeConfig> {
        let elem = self.search_element(tag_name)?;
        elem.attribute
            .iter()
            .chain(self.global_attribute.iter())
            .find(|x| x.name == attr_name)
    }

    pub(crate) fn search_property(
        &self,
        tag_name: &str,
        attr_name: &str,
    ) -> Option<&AttributeConfig> {
        let comp = self.search_component(tag_name)?;
        comp.property
            .iter()
            .chain(self.global_attribute.iter())
            .find(|x| x.name == attr_name)
    }

    pub(crate) fn list_attributes(
        &self,
        tag_name: &str,
    ) -> Option<impl Iterator<Item = &AttributeConfig>> {
        let elem = self.search_element(tag_name)?;
        Some(elem.attribute.iter().chain(self.global_attribute.iter()))
    }

    pub(crate) fn list_properties(
        &self,
        tag_name: &str,
    ) -> Option<impl Iterator<Item = &AttributeConfig>> {
        let comp = self.search_component(tag_name)?;
        Some(comp.property.iter().chain(self.global_attribute.iter()))
    }

    pub(crate) fn search_global_event(&self, event_name: &str) -> Option<&EventConfig> {
        self.global_event.iter().find(|x| x.name == event_name)
    }

    pub(crate) fn search_element_event(
        &self,
        tag_name: &str,
        event_name: &str,
    ) -> Option<&EventConfig> {
        let elem = self.search_element(tag_name)?;
        elem.event
            .iter()
            .chain(self.global_event.iter())
            .find(|x| x.name == event_name)
    }

    pub(crate) fn search_component_event(
        &self,
        tag_name: &str,
        event_name: &str,
    ) -> Option<&EventConfig> {
        let comp = self.search_component(tag_name)?;
        comp.event
            .iter()
            .chain(self.global_event.iter())
            .find(|x| x.name == event_name)
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
    pub(crate) ty: MediaFeatureType,
    #[serde(default)]
    pub(crate) options: Vec<String>,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum MediaFeatureType {
    Any,
    Range,
}

impl Default for MediaFeatureType {
    fn default() -> Self {
        Self::Any
    }
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
    pub(crate) options: Vec<String>,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
}

impl BackendConfig {
    pub(crate) fn extract_template_backend_config(&self, mut w: impl Write) -> std::fmt::Result {
        let config = self;

        // write header
        writeln!(
            w,
            r#"export type GlassEaselTemplateBackendConfig = {{
    name: {:?}
    description: {:?}
    majorVersion: {}
    minorVersion: {}
}}
"#,
            config.glass_easel_backend_config.name,
            config.glass_easel_backend_config.description,
            config.glass_easel_backend_config.major_version,
            config.glass_easel_backend_config.minor_version,
        )?;

        // write global attributes
        writeln!(w, r#"type GlobalAttributes = {{"#)?;
        for attr in &config.global_attribute {
            let ty = if attr.ty.is_empty() { "any" } else { &attr.ty };
            writeln!(w, r#"{:?}: {}"#, dash_to_camel(&attr.name), ty)?;
        }
        writeln!(w, r#"}}"#)?;

        // write properties per component
        writeln!(w, r#"export type ComponentProperties = {{"#)?;
        for elem in &config.element {
            writeln!(w, r#"{:?}: GlobalAttributes & {{"#, elem.tag_name)?;
            for attr in &elem.attribute {
                let ty = if attr.ty.is_empty() { "any" } else { &attr.ty };
                writeln!(w, r#"{:?}: {}"#, dash_to_camel(&attr.name), ty)?;
            }
            writeln!(w, r#"}}"#)?;
        }
        for comp in &config.component {
            writeln!(w, r#"{:?}: GlobalAttributes & {{"#, comp.tag_name)?;
            for prop in &comp.property {
                let ty = if prop.ty.is_empty() { "any" } else { &prop.ty };
                writeln!(w, r#"{:?}: {}"#, dash_to_camel(&prop.name), ty)?;
            }
            writeln!(w, r#"}}"#)?;
        }
        writeln!(w, r#"}}"#)?;
        Ok(())
    }

    pub(crate) fn generate_template_backend_config(&self) -> String {
        let mut template_backend_config = String::new();
        self.extract_template_backend_config(&mut template_backend_config)
            .unwrap_or_else(|err| {
                template_backend_config = String::new();
                log::error!("Failed to extract template backend configuration: {}", err);
            });
        template_backend_config
    }
}
