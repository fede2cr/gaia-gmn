//! Nights browser – shows all capture sessions with summary statistics.

use leptos::*;

use crate::components::night_card::NightCard;
use crate::server_fns::get_nights;

#[component]
pub fn NightsPage() -> impl IntoView {
    let nights = create_resource(|| (), |_| get_nights());

    view! {
        <section class="nights-page">
            <h1>"Capture Nights"</h1>
            <p class="subtitle">
                "Browse all observation nights. Each directory contains FF "
                "(compressed frame) and FS (field sum) files captured during "
                "a single night session."
            </p>

            <Suspense fallback=|| view! { <p class="loading">"Scanning nights…"</p> }>
                {move || {
                    nights.get().map(|res| match res {
                        Ok(list) if list.is_empty() => view! {
                            <div class="empty-state">
                                <p>"No capture nights found."</p>
                                <p class="hint">
                                    "Night directories appear after the capture "
                                    "server records its first frames."
                                </p>
                            </div>
                        }.into_view(),
                        Ok(list) => view! {
                            <p class="night-count">{list.len()} " night(s) on disk"</p>
                            <div class="nights-grid">
                                {list.into_iter().map(|n| {
                                    view! { <NightCard summary=n /> }
                                }).collect_view()}
                            </div>
                        }.into_view(),
                        Err(e) => view! {
                            <p class="error">"Error: " {e.to_string()}</p>
                        }.into_view(),
                    })
                }}
            </Suspense>
        </section>
    }
}
