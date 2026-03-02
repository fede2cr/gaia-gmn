//! Night card component – summary card for a single capture night.

use leptos::*;

use crate::model::NightSummary;

#[component]
pub fn NightCard(summary: NightSummary) -> impl IntoView {
    let href = format!("/nights/{}", summary.dir_name);
    let size_str = summary.human_size();

    view! {
        <a href=href class="card night-card">
            <div class="night-card-header">
                <span class="night-station">{&summary.station_id}</span>
                <span class="night-time">{&summary.start_time}</span>
            </div>
            <div class="night-card-stats">
                <div class="night-stat-item">
                    <span class="night-stat-value">{summary.ff_count}</span>
                    <span class="night-stat-label">"FF files"</span>
                </div>
                <div class="night-stat-item">
                    <span class="night-stat-value">{summary.fs_count}</span>
                    <span class="night-stat-label">"FS files"</span>
                </div>
                <div class="night-stat-item">
                    <span class="night-stat-value">{size_str}</span>
                    <span class="night-stat-label">"Total"</span>
                </div>
            </div>
        </a>
    }
}
