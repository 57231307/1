use yew::prelude::*;
use crate::components::navigation::Navigation;
use crate::components::header::Header;

#[derive(Properties, PartialEq)]
pub struct MainLayoutProps {
    pub children: Children,
}

#[function_component(MainLayout)]
pub fn main_layout(props: &MainLayoutProps) -> Html {
    let collapsed = use_state(|| false);
    let on_toggle = {
        let collapsed = collapsed.clone();
        Callback::from(move |_| {
            collapsed.set(!*collapsed);
        })
    };

    html! {
        <div class="main-layout">
            <Navigation collapsed={*collapsed} on_toggle={on_toggle} />
            <div class={classes!("main-content", (*collapsed).then_some("expanded"))}>
                <Header />
                <main class="content-area">
                    { props.children.clone() }
                </main>
            </div>
        </div>
    }
}
