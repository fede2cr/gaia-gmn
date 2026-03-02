//! Capture page – live camera feed, real-time stats, and FF file browser.

use leptos::*;

use crate::components::file_list::FileList;
use crate::components::live_preview::LivePreview;
use crate::components::status_card::StatusCard;
use crate::server_fns::{get_capture_files, get_capture_status};

#[component]
pub fn CapturePage() -> impl IntoView {
    let status = create_resource(|| (), |_| get_capture_status());
    let files = create_resource(|| (), |_| get_capture_files());

    view! {
        <section class="capture-page">
            <h1>"Capture"</h1>

            <div class="capture-grid">
                // ── Live preview ──────────────────────────────
                <div class="capture-preview">
                    <h2>"Live Camera Feed"</h2>
                    <LivePreview />
                </div>

                // ── Status card ───────────────────────────────
                <div class="capture-status">
                    <h2>"Statistics"</h2>
                    <Suspense fallback=|| view! { <p class="loading">"Loading…"</p> }>
                        {move || {
                            status.get().map(|res| match res {
                                Ok(s) => view! { <StatusCard status=s /> }.into_view(),
                                Err(e) => view! {
                                    <p class="error">"Error: " {e.to_string()}</p>
                                }.into_view(),
                            })
                        }}
                    </Suspense>
                </div>
            </div>

            // ── FF file list ──────────────────────────────────
            <div class="capture-files">
                <h2>"Captured FF Files"</h2>
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
            </div>
        </section>
    }
}
