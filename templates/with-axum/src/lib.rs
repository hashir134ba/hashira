mod components;

use crate::components::{root_layout, Counter};
use hashira::{
    app::{App as HashiraApp, AppService, RenderContext},
    page_component,
    server::{Metadata, PageLinks, LinkTag},
};
use serde::{Deserialize, Serialize};
use yew::{html::ChildrenProps, BaseComponent, Properties};

#[page_component]
pub fn App(props: &ChildrenProps) -> yew::Html {
    yew::html! {
       <>
        <header>
            <nav>
                <a href="/">{"Home"}</a>
                <a href="/counter">{"Counter"}</a>
            </nav>
        </header>
        <>{for props.children.iter()}</>
       </>
    }
}

#[page_component]
pub fn HomePage() -> yew::Html {
    yew::html! {
        <div class="container">
            <div class="logo-container">
            <span class="hashira" title="Hashira">{"Hashira"}</span>
            <span class="divider">{'\u{00D7}'}</span>
            <a href="https://crates.io/crates/axum" target="_blank" rel="noopener">
                <img title="Axum" alt="Axum" src="https://raw.githubusercontent.com/tokio-rs/website/master/public/img/icons/tokio.svg"/>
            </a>
        </div>
        </div>
    }
}

#[derive(PartialEq, Clone, Properties, Serialize, Deserialize)]
pub struct CounterPageProps {
    #[prop_or_default]
    counter_start: i32,
}

#[page_component]
pub fn CounterPage(props: &CounterPageProps) -> yew::Html {
    yew::html! {
        <div class="container">
            <Counter value={props.counter_start}/>
        </div>
    }
}

// Setup all the components
pub fn hashira<C>() -> AppService
where
    C: BaseComponent<Properties = ChildrenProps>,
{
    HashiraApp::<C>::new()
        .use_default_error_pages()
        .layout(root_layout)
        .page("/", |mut ctx: RenderContext| async {
            ctx.metadata(Metadata::new().description("An Hashira x Actix Web example"));
            ctx.links(PageLinks::new().insert(LinkTag::stylesheet("/static/global.css")));
            let res = ctx.render::<HomePage, C>().await;
            Ok(res)
        })
        .page("/counter", |mut ctx: RenderContext| async {
            ctx.title("Hashira | Counter");
            ctx.metadata(Metadata::new().description("A counter made with hashira actix-web"));
            ctx.links(PageLinks::new().insert(LinkTag::stylesheet("/static/global.css")));
            let props = yew::props! { CounterPageProps {} };
            let res = ctx.render_with_props::<CounterPage, C>(props).await;
            Ok(res)
        })
        .build()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    wasm_logger::init(wasm_logger::Config::default());
    log::debug!("Hydrating app...");
    let service = hashira::<App>();
    hashira::client::mount::<App>(service);
}