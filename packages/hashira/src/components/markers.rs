// TODO: append an uuid to ensure the value is unique?

pub const HASHIRA_META_MARKER: &str = "__hashira__meta__";
pub const HASHIRA_CONTENT_MARKER: &str = "__hashira__content__";
pub const HASHIRA_LINKS_MARKER: &str = "__hashira__links__";
pub const HASHIRA_SCRIPTS_MARKER: &str = "__hashira__scripts__";
pub const HASHIRA_ROOT: &str = "__hashira__root__";

#[yew::function_component]
pub fn Meta() -> yew::Html {
    return yew::html! {
        {HASHIRA_META_MARKER}
    };
}

#[yew::function_component]
pub fn Content() -> yew::Html {
    return yew::html! {
        {HASHIRA_CONTENT_MARKER}
    };
}

#[yew::function_component]
pub fn Links() -> yew::Html {
    return yew::html! {
        {HASHIRA_LINKS_MARKER}
    };
}

#[yew::function_component]
pub fn Scripts() -> yew::Html {
    return yew::html! {
        {HASHIRA_SCRIPTS_MARKER}
    };
}