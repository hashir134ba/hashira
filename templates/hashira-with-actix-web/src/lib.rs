mod components;

use crate::components::{root_layout, Counter, HashiraActixWeb, ThemeToggle};
use hashira::{
    app::{App as HashiraApp, AppService, RenderContext},
    server::{LinkTag, Metadata, PageLinks},
};
use serde::{Deserialize, Serialize};
use yew::{function_component, html::ChildrenProps, BaseComponent, Properties};

#[yew::function_component]
pub fn App(props: &ChildrenProps) -> yew::Html {
    yew::html! {
       <>
        <header>
            <nav>
                <a href="/">{"Home"}</a>
                <a href="/counter">{"Counter"}</a>
                <div class="theme-toggle">
                    <ThemeToggle/>
                </div>
            </nav>
        </header>
        <>{for props.children.iter()}</>
       </>
    }
}

#[function_component]
pub fn HomePage() -> yew::Html {
    yew::html! {
        <div class="container">
            <HashiraActixWeb/>
        </div>
    }
}

#[derive(PartialEq, Clone, Properties, Serialize, Deserialize)]
pub struct CounterPageProps {
    #[prop_or_default]
    counter_start: i32,
}

#[yew::function_component]
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
        .page("/", |mut ctx: RenderContext<HomePage, C>| async {
            ctx.add_title("Hashira");
            ctx.add_links(PageLinks::new().add(LinkTag::stylesheet("/static/global.css")));
            ctx.add_metadata(Metadata::new().description("An Hashira x Actix Web example"));

            let res = ctx.render().await;
            Ok(res)
        })
        .page("/counter", |mut ctx: RenderContext<CounterPage, C>| async {
            ctx.add_title("Hashira | Counter");
            ctx.add_links(PageLinks::new().add(LinkTag::stylesheet("/static/global.css")));
            ctx.add_metadata(Metadata::new().description("A counter made with hashira actix-web"));

            let props = yew::props! { CounterPageProps {} };
            let res = ctx.render_with_props(props).await;
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