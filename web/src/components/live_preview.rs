//! Live preview – loads the latest camera JPEG from the capture server.
//!
//! The image is refreshed every 2 seconds by appending a cache-busting
//! query parameter.

use leptos::*;

/// URL where the capture server's live.jpg can be reached.
/// In the container stack this goes through the compose network.
const LIVE_IMG_PATH: &str = "/data/live.jpg";

/// Displays the latest camera frame, auto-refreshing.
#[component]
pub fn LivePreview() -> impl IntoView {
    let (tick, _set_tick) = create_signal(0u64);

    // Bump a counter every 2 s to force a new image URL.
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;

        let cb = Closure::wrap(Box::new(move || {
            set_tick.update(|t| *t += 1);
        }) as Box<dyn Fn()>);

        // `setInterval` returns an ID we intentionally leak because it lives
        // for the lifetime of the page.
        let _ = web_sys::window()
            .unwrap()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                2_000,
            );
        cb.forget();
    }

    let img_url = move || format!("{}?t={}", LIVE_IMG_PATH, tick.get());

    view! {
        <div class="live-preview">
            <img
                src=img_url
                alt="Live camera frame"
                class="live-img"
            />
            <p class="live-caption">"Auto-refreshes every 2 s"</p>
        </div>
    }
}
