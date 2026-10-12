//! Under the lists: where the batch stands, what the last action did, the
//! summary of the last rename with its export, and the confirmation dialog.

use colony_ui::Typography;
use colony_ui::theme::Palette;
use iced::widget::{column, row, scrollable, space, text};
use iced::{Alignment, Element, Length};

use super::{Confirm, Feedback, FetchState, Message, Rename, RenameSummary};
use crate::export::ExportFormat;
use crate::i18n::{t, tf};
use crate::rename::RenameOutcome;
use crate::ui::widgets::{self, Tone, icons};

pub fn view<'a>(state: &'a Rename, typo: &Typography) -> Element<'a, Message> {
    let mut page = column![status(state, typo)].spacing(8);
    if let Some(feedback) = &state.feedback {
        page = page.push(widgets::label(
            typo,
            feedback_text(feedback),
            13,
            Palette::TEXT_PRIMARY(),
        ));
    }
    if !state.summaries.is_empty() {
        page = page.push(table(state, typo));
    }
    page.into()
}

fn status<'a>(state: &Rename, typo: &Typography) -> Element<'a, Message> {
    let (message, color) = match &state.fetch {
        FetchState::Loading => (t("status_loading").to_string(), Palette::ACCENT()),
        FetchState::Failed(err) => (
            tf(
                "status_error",
                &[("error", &super::preview::fetch_error_text(err))],
            ),
            Palette::ERROR(),
        ),
        _ if state.matching => (t("status_loading").to_string(), Palette::ACCENT()),
        _ if state.results.is_empty() => (t("status_empty").to_string(), Palette::TEXT_MUTED()),
        _ => (
            tf("status_ready", &[("count", &state.results.len())]),
            Palette::SUCCESS(),
        ),
    };
    widgets::label(typo, message, 13, color).into()
}

fn feedback_text(feedback: &Feedback) -> String {
    match feedback {
        Feedback::NothingToRename => t("feedback_nothing_to_rename").to_string(),
        Feedback::Renamed {
            renamed,
            skipped,
            failed,
        } => tf(
            "feedback_renamed",
            &[
                ("renamed", renamed),
                ("skipped", skipped),
                ("failed", failed),
            ],
        ),
        Feedback::Previewed {
            ready,
            skipped,
            failed,
        } => tf(
            "feedback_previewed",
            &[("ready", ready), ("skipped", skipped), ("failed", failed)],
        ),
        Feedback::NothingToExport => t("feedback_nothing_to_export").to_string(),
        Feedback::Exported(path) => tf("feedback_exported", &[("path", &path.display())]),
        Feedback::ExportFailed(error) => tf("feedback_export_failed", &[("error", error)]),
    }
}

/// What happened to one file, in the interface language.
pub fn message(summary: &RenameSummary) -> String {
    let path = &summary.resolved;
    let key = |plain, adjusted| {
        if summary.collision_adjusted {
            adjusted
        } else {
            plain
        }
    };
    match &summary.outcome {
        RenameOutcome::Renamed => tf(
            key("summary_renamed", "summary_renamed_adjusted"),
            &[("path", path)],
        ),
        RenameOutcome::DryRun => tf(
            key("summary_dry_run", "summary_dry_run_adjusted"),
            &[("path", path)],
        ),
        RenameOutcome::Unchanged => tf("summary_unchanged", &[("path", path)]),
        RenameOutcome::Skipped(reason) => tf("summary_skipped", &[("reason", reason)]),
        RenameOutcome::Failed(reason) => tf("summary_failed", &[("reason", reason)]),
    }
}

/// The colour a summary line is written in.
pub fn color(outcome: &RenameOutcome) -> iced::Color {
    match outcome {
        RenameOutcome::Renamed => Palette::SUCCESS(),
        RenameOutcome::DryRun => Palette::WARNING(),
        RenameOutcome::Unchanged | RenameOutcome::Skipped(_) => Palette::TEXT_MUTED(),
        RenameOutcome::Failed(_) => Palette::ERROR(),
    }
}

fn table<'a>(state: &'a Rename, typo: &Typography) -> Element<'a, Message> {
    let header = row![
        widgets::heading(typo, t("summary_title"), 14),
        space::horizontal(),
        widgets::action(
            typo,
            icons::EXPORT,
            t("summary_export_csv"),
            Tone::Default,
            Some(Message::Export(ExportFormat::Csv)),
        ),
        widgets::action(
            typo,
            icons::EXPORT,
            t("summary_export_json"),
            Tone::Default,
            Some(Message::Export(ExportFormat::Json)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let cell = |content: String, size: u16, color, portion| {
        text(content)
            .size(typo.sz(size))
            .font(typo.regular)
            .color(color)
            .width(Length::FillPortion(portion))
    };
    let columns = row![
        cell(
            t("summary_status").to_string(),
            12,
            Palette::TEXT_MUTED(),
            1
        ),
        cell(
            t("summary_message").to_string(),
            12,
            Palette::TEXT_MUTED(),
            3
        ),
        cell(
            t("summary_final_path").to_string(),
            12,
            Palette::TEXT_MUTED(),
            4
        ),
    ]
    .spacing(8);
    let rows = state.summaries.iter().map(|summary| {
        let status = if summary.failed() {
            t("summary_status_error")
        } else {
            t("summary_status_ok")
        };
        row![
            cell(status.to_string(), 12, color(&summary.outcome), 1),
            cell(message(summary), 12, Palette::TEXT_PRIMARY(), 3),
            cell(summary.resolved.clone(), 12, Palette::TEXT_SECONDARY(), 4),
        ]
        .spacing(8)
        .into()
    });

    widgets::card(
        column![
            header,
            columns,
            scrollable(column(rows).spacing(4)).height(Length::Shrink),
        ]
        .spacing(8),
    )
    .max_height(260.0 * typo.scale)
    .into()
}

/// The dialog asking to confirm a rename or clearing the list.
pub fn confirm_dialog<'a>(
    state: &Rename,
    typo: &Typography,
    confirm: Confirm,
) -> Element<'a, Message> {
    let (title, body, action, tone) = match confirm {
        Confirm::Rename => (
            t("confirm_rename_title"),
            tf("confirm_rename_body", &[("count", &state.results.len())]),
            t("confirm_rename_action"),
            Tone::Primary,
        ),
        Confirm::Clear => (
            t("confirm_clear_title"),
            tf("confirm_clear_body", &[("count", &state.files.len())]),
            t("confirm_clear_action"),
            Tone::Danger,
        ),
    };
    let buttons = row![
        space::horizontal(),
        widgets::text_button(
            typo,
            t("confirm_cancel"),
            Tone::Default,
            Some(Message::Cancel)
        ),
        widgets::text_button(typo, action, tone, Some(Message::Confirm)),
    ]
    .spacing(8);
    widgets::dialog(
        typo,
        title,
        column![
            widgets::label(typo, body, 13, Palette::TEXT_SECONDARY()),
            widgets::label(typo, t("confirm_hint"), 11, Palette::TEXT_DIMMER()),
            buttons,
        ]
        .spacing(12),
        460.0,
    )
}
