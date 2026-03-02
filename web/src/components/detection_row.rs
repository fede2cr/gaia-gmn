//! Detection table component – renders meteor detections in a data table.

use leptos::*;

use crate::model::MeteorDetection;

/// Render a sortable table of meteor detections.
#[component]
pub fn DetectionTable(detections: Vec<MeteorDetection>) -> impl IntoView {
    view! {
        <div class="detection-table-wrap">
            <table class="detection-table">
                <thead>
                    <tr>
                        <th>"Time (UTC)"</th>
                        <th>"FF File"</th>
                        <th>"Mag"</th>
                        <th>"Duration"</th>
                        <th>"Frames"</th>
                        <th>"RA"</th>
                        <th>"Dec"</th>
                        <th>"Status"</th>
                    </tr>
                </thead>
                <tbody>
                    {detections
                        .into_iter()
                        .map(|d| view! { <DetectionRow det=d /> })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn DetectionRow(det: MeteorDetection) -> impl IntoView {
    let night_link = format!("/nights/{}", det.night_dir);
    let mag_str = det
        .magnitude
        .map(|m| format!("{m:.1}"))
        .unwrap_or_else(|| "—".into());
    let dur_str = det
        .duration_secs
        .map(|d| format!("{d:.2}s"))
        .unwrap_or_else(|| "—".into());
    let frames_str = det
        .num_frames
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into());
    let ra_str = det
        .ra_deg
        .map(|r| format!("{r:.2}°"))
        .unwrap_or_else(|| "—".into());
    let dec_str = det
        .dec_deg
        .map(|d| format!("{d:.2}°"))
        .unwrap_or_else(|| "—".into());
    let status_class = if det.confirmed {
        "badge badge-confirmed"
    } else {
        "badge badge-candidate"
    };
    let status_text = if det.confirmed {
        "Confirmed"
    } else {
        "Candidate"
    };

    view! {
        <tr class="detection-row">
            <td class="det-time">
                <a href=night_link>{&det.timestamp}</a>
            </td>
            <td class="det-ff mono">{&det.ff_file}</td>
            <td class="det-mag">{mag_str}</td>
            <td class="det-dur">{dur_str}</td>
            <td class="det-frames">{frames_str}</td>
            <td class="det-ra mono">{ra_str}</td>
            <td class="det-dec mono">{dec_str}</td>
            <td><span class=status_class>{status_text}</span></td>
        </tr>
    }
}
