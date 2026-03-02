//! File list component – renders a table of FF/FS files with download links.

use leptos::*;

use crate::model::{human_size, FileEntry};

#[component]
pub fn FileList(files: Vec<FileEntry>) -> impl IntoView {
    if files.is_empty() {
        return view! {
            <p class="no-files">"No FF/FS files captured yet."</p>
        }
        .into_view();
    }

    let ff_count = files.iter().filter(|f| f.is_ff()).count();
    let fs_count = files.iter().filter(|f| f.is_fs()).count();
    let total: u64 = files.iter().map(|f| f.size).sum();

    view! {
        <div class="file-list">
            <p class="file-summary">
                {ff_count} " FF · " {fs_count} " FS · " {human_size(total)} " total"
            </p>
            <table class="file-table">
                <thead>
                    <tr>
                        <th>"Type"</th>
                        <th>"Name"</th>
                        <th>"Size"</th>
                    </tr>
                </thead>
                <tbody>
                    {files
                        .into_iter()
                        .map(|f| {
                            let badge_class = if f.is_ff() {
                                "badge badge-ff"
                            } else {
                                "badge badge-fs"
                            };
                            let badge_text = if f.is_ff() { "FF" } else { "FS" };
                            let size_str = f.human_size();
                            view! {
                                <tr>
                                    <td><span class=badge_class>{badge_text}</span></td>
                                    <td class="file-name">{&f.name}</td>
                                    <td class="file-size">{size_str}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
    .into_view()
}
