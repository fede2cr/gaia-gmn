//! Night detail page – shows all FF/FS files and detections for one night.

use leptos::*;
use leptos_router::*;

use crate::components::detection_row::DetectionTable;
use crate::components::file_list::FileList;
use crate::model::human_size;
use crate::server_fns::{get_night_detections, get_night_files};

#[component]
pub fn NightDetailPage() -> impl IntoView {
    let params = use_params_map();
    let night_dir = move || {
        params.with(|p| p.get("night_dir").cloned().unwrap_or_default())
    };

    let files = create_resource(night_dir, |dir| get_night_files(dir));
    let detections = create_resource(night_dir, |dir| get_night_detections(dir));

    view! {
        <section class="night-detail-page">
            <div class="page-header">
                <a href="/nights" class="back-link">"← Nights"</a>
                <h1>{night_dir}</h1>
            </div>

            // ── Summary bar ────────────────────────────────────────
            <Suspense fallback=|| view! { <p class="loading">"Loading files…"</p> }>
                {move || {
                    files.get().map(|res| match res {
                        Ok(ref f) => {
                            let ff_count = f.iter().filter(|e| e.is_ff()).count();
                            let fs_count = f.iter().filter(|e| e.is_fs()).count();
                            let total: u64 = f.iter().map(|e| e.size).sum();
                            view! {
                                <div class="night-summary-bar">
                                    <span class="night-stat">{ff_count} " FF files"</span>
                                    <span class="night-stat">{fs_count} " FS files"</span>
                                    <span class="night-stat">{human_size(total)}</span>
                                </div>
                            }.into_view()
                        }
                        Err(_) => view! {}.into_view(),
                    })
                }}
            </Suspense>

            // ── Detections ─────────────────────────────────────────
            <h2>"Detections"</h2>
            <Suspense fallback=|| view! { <p class="loading">"Loading detections…"</p> }>
                {move || {
                    detections.get().map(|res| match res {
                        Ok(dets) if dets.is_empty() => view! {
                            <p class="placeholder-msg">
                                "No detections recorded for this night yet. "
                                "Detections appear after the processing pipeline "
                                "analyses the captured FF files."
                            </p>
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

            // ── Files ──────────────────────────────────────────────
            <h2>"Captured Files"</h2>
            <Suspense fallback=|| view! { <p class="loading">"Loading files…"</p> }>
                {move || {
                    files.get().map(|res| match res {
                        Ok(f) => view! { <FileList files=f /> }.into_view(),
                        Err(e) => view! {
                            <p class="error">"Error: " {e.to_string()}</p>
                        }.into_view(),
                    })
                }}
            </Suspense>
        </section>
    }
}
