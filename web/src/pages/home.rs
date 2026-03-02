//! Home / dashboard page – shows capture status, station info, live
//! preview, and a quick summary of detection activity.

use leptos::*;

use crate::components::live_preview::LivePreview;
use crate::components::station_card::StationCard;
use crate::components::status_card::StatusCard;
use crate::server_fns::{get_capture_status, get_detection_counts, get_nights, get_station_info};

#[component]
pub fn Home() -> impl IntoView {
    let status = create_resource(|| (), |_| get_capture_status());
    let station = create_resource(|| (), |_| get_station_info());
    let det_counts = create_resource(|| (), |_| get_detection_counts());
    let nights = create_resource(|| (), |_| get_nights());

    view! {
        <section class="dashboard">
            <h1>"Gaia GMN – Meteor Detection"</h1>
            <p class="subtitle">
                "Global Meteor Network capture and processing dashboard."
            </p>

            <div class="dashboard-grid">
                // ── Capture status ────────────────────────────
                <Suspense fallback=|| view! { <p class="loading">"Loading status…"</p> }>
                    {move || {
                        status.get().map(|res| match res {
                            Ok(s) => view! { <StatusCard status=s /> }.into_view(),
                            Err(e) => view! {
                                <div class="card error-card">
                                    <p>"Capture server offline: " {e.to_string()}</p>
                                </div>
                            }.into_view(),
                        })
                    }}
                </Suspense>

                <LivePreview />
            </div>

            // ── Quick stats row ───────────────────────────────
            <div class="quick-stats">
                <Suspense fallback=|| view! {}>
                    {move || {
                        nights.get().map(|res| match res {
                            Ok(n) => view! {
                                <a href="/nights" class="card quick-stat-card">
                                    <span class="stat-value">{n.len()}</span>
                                    <span class="stat-label">"Capture Nights"</span>
                                </a>
                            }.into_view(),
                            Err(_) => view! {}.into_view(),
                        })
                    }}
                </Suspense>
                <Suspense fallback=|| view! {}>
                    {move || {
                        det_counts.get().map(|res| match res {
                            Ok((total, confirmed)) => view! {
                                <a href="/detections" class="card quick-stat-card">
                                    <span class="stat-value">{total}</span>
                                    <span class="stat-label">"Detections"</span>
                                </a>
                                <div class="card quick-stat-card">
                                    <span class="stat-value confirmed">{confirmed}</span>
                                    <span class="stat-label">"Confirmed"</span>
                                </div>
                            }.into_view(),
                            Err(_) => view! {}.into_view(),
                        })
                    }}
                </Suspense>
            </div>

            // ── Station info ──────────────────────────────────
            <h2>"Station"</h2>
            <Suspense fallback=|| view! { <p class="loading">"Loading station info…"</p> }>
                {move || {
                    station.get().map(|res| match res {
                        Ok(info) => view! { <StationCard info=info /> }.into_view(),
                        Err(_) => view! {
                            <p class="hint">"Station info unavailable."</p>
                        }.into_view(),
                    })
                }}
            </Suspense>
        </section>
    }
}
