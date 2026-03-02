//! Root Leptos application component with routing.

use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::pages::{
    capture::CapturePage, detections::DetectionsPage, home::Home,
    night_detail::NightDetailPage, nights::NightsPage,
};

/// The root `<App/>` component.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/gaia-gmn-web.css"/>
        <Title text="Gaia GMN – Meteor Detection"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>

        <Router>
            <nav class="top-nav">
                <a href="/" class="nav-brand">"☄ Gaia GMN"</a>
                <div class="nav-links">
                    <a href="/">"Dashboard"</a>
                    <a href="/capture">"Capture"</a>
                    <a href="/nights">"Nights"</a>
                    <a href="/detections">"Detections"</a>
                </div>
            </nav>
            <main class="main-content">
                <Routes>
                    <Route path="/" view=Home/>
                    <Route path="/capture" view=CapturePage/>
                    <Route path="/nights" view=NightsPage/>
                    <Route path="/nights/:night_dir" view=NightDetailPage/>
                    <Route path="/detections" view=DetectionsPage/>
                </Routes>
            </main>
        </Router>
    }
}
