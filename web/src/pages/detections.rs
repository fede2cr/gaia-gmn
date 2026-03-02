//! Detections page – meteor detection results from the processing pipeline.

use leptos::*;

use crate::components::detection_row::DetectionTable;
use crate::server_fns::{get_detection_counts, get_recent_detections};

#[component]
pub fn DetectionsPage() -> impl IntoView {
    let counts = create_resource(|| (), |_| get_detection_counts());
    let detections = create_resource(|| (), |_| get_recent_detections(Some(100)));

    view! {
        <section class="detections-page">
            <h1>"Detections"</h1>
            <p class="subtitle">
                "Meteor detection results from the processing pipeline. "
                "Each row represents a candidate or confirmed meteor event "
                "extracted from the captured FF files."
            </p>

            // ── Summary counts ────────────────────────────────────
            <Suspense fallback=|| view! {}>
                {move || {
                    counts.get().map(|res| match res {
                        Ok((total, confirmed)) => view! {
                            <div class="detection-summary">
                                <div class="det-stat">
                                    <span class="det-stat-value">{total}</span>
                                    <span class="det-stat-label">"Total"</span>
                                </div>
                                <div class="det-stat">
                                    <span class="det-stat-value confirmed">{confirmed}</span>
                                    <span class="det-stat-label">"Confirmed"</span>
                                </div>
                                <div class="det-stat">
                                    <span class="det-stat-value candidate">{total - confirmed}</span>
                                    <span class="det-stat-label">"Candidates"</span>
                                </div>
                            </div>
                        }.into_view(),
                        Err(_) => view! {}.into_view(),
                    })
                }}
            </Suspense>

            // ── Detection list ────────────────────────────────────
            <Suspense fallback=|| view! { <p class="loading">"Loading detections…"</p> }>
                {move || {
                    detections.get().map(|res| match res {
                        Ok(dets) if dets.is_empty() => view! {
                            <div class="empty-state">
                                <p class="placeholder-msg">
                                    "No detections recorded yet. Detections will appear "
                                    "once the processing pipeline analyses captured FF files "
                                    "and identifies meteor trajectories. ☄"
                                </p>
                            </div>
                        }.into_view(),
                        Ok(dets) => view! {
                            <DetectionTable detections=dets />
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
