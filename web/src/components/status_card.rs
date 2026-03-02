//! Status card – shows capture uptime, frame count, and FF file count.

use leptos::*;

use crate::model::CaptureStatus;

#[component]
pub fn StatusCard(status: CaptureStatus) -> impl IntoView {
    let hours = status.uptime_secs / 3600;
    let mins = (status.uptime_secs % 3600) / 60;
    let uptime_fmt = format!("{hours}h {mins}m");

    view! {
        <div class="card status-card">
            <div class="stat">
                <span class="stat-value">{uptime_fmt}</span>
                <span class="stat-label">"Uptime"</span>
            </div>
            <div class="stat">
                <span class="stat-value">{status.total_frames}</span>
                <span class="stat-label">"Total Frames"</span>
            </div>
            <div class="stat">
                <span class="stat-value">{status.ff_files_written}</span>
                <span class="stat-label">"FF Files Written"</span>
            </div>
        </div>
    }
}
