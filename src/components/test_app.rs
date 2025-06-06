use yew::prelude::*;

pub struct TestApp;

impl Component for TestApp {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <div>
                <h1>{ "Test App - If you see this, Yew is working!" }</h1>
                <p>{ "The blank screen issue is in the main App component." }</p>
            </div>
        }
    }
}
