use lsp_types::Url;

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct BackendConfig {
    pub(crate) element: Vec<ElementConfig>,
    pub(crate) component: Vec<ElementConfig>,
    pub(crate) global_attribute: Vec<AttributeConfig>,
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
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct AttributeConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct EventConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) reference: Option<Url>,
}

impl BackendConfig {
    pub(crate) fn search_element(&self, tag_name: &str) -> Option<&ElementConfig> {
        self.element.iter().find(|x| x.tag_name == tag_name)
    }

    pub(crate) fn search_component(&self, tag_name: &str) -> Option<&ElementConfig> {
        self.component.iter().find(|x| x.tag_name == tag_name)
    }
}
