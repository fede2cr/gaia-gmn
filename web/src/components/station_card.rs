//! Station info card component – shows capture station configuration.

use leptos::*;

use crate::model::StationInfo;

#[component]
pub fn StationCard(info: StationInfo) -> impl IntoView {
    let loc = format!("{:.4}°, {:.4}° ({:.0} m)", info.latitude, info.longitude, info.elevation);

    view! {
        <div class="card station-card">
            <h3 class="station-id">{&info.station_id}</h3>
            <div class="station-details">
                <div class="station-row">
                    <span class="station-label">"Location"</span>
                    <span class="station-value">{loc}</span>
                </div>
                <div class="station-row">
                    <span class="station-label">"Resolution"</span>
                    <span class="station-value">{&info.resolution}</span>
                </div>
                <div class="station-row">
                    <span class="station-label">"Frame Rate"</span>
                    <span class="station-value">{format!("{:.1} fps", info.fps)}</span>
                </div>
                <div class="station-row">
                    <span class="station-label">"Frames/FF"</span>
                    <span class="station-value">{info.ff_nframes}</span>
                </div>
            </div>
        </div>
    }
}
