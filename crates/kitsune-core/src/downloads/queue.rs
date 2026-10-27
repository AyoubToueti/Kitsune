//! Pure scheduling policy for permanent downloads.
//!
//! librqbit can include or exclude files, but it does not expose a per-torrent
//! priority weight. The queue therefore controls priority by pausing all but
//! the best N eligible downloads.

use super::store::{DownloadRecord, DownloadStatus};
use std::collections::HashMap;

/// The scheduler's desired state for one record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueAction {
    Start,
    Queue,
}

/// Pick at most `max_concurrent` eligible downloads.
///
/// Higher priority wins, then older records. User-paused records are never
/// selected automatically. Completed and failed records are ignored.
pub fn plan_queue(records: &[DownloadRecord], max_concurrent: u32) -> Vec<(u64, QueueAction)> {
    // A season-pack selection creates several records that share one torrent.
    // Schedule that torrent as one unit, or pausing the "second episode" would
    // pause the first episode too and leave the records disagreeing about the
    // same engine state.
    let mut groups: HashMap<String, Vec<&DownloadRecord>> = HashMap::new();
    for record in records.iter().filter(|record| {
        !record.user_paused
            && matches!(
                record.status,
                DownloadStatus::Downloading | DownloadStatus::Queued
            )
    }) {
        // Group by the stable torrent identity, not the engine id: a record
        // restored from an earlier process keeps its old id until the reattach
        // finishes, and two rows of one season pack must still share a slot.
        groups.entry(record.group_key()).or_default().push(record);
    }

    let mut groups: Vec<Vec<&DownloadRecord>> = groups.into_values().collect();
    groups.sort_by_key(|group| {
        group
            .iter()
            .map(|record| (record.priority.rank(), record.added_at, record.id))
            .min()
            .unwrap_or((u8::MAX, i64::MAX, u64::MAX))
    });
    let limit = max_concurrent.max(1) as usize;

    let planned: Vec<(u64, QueueAction)> = groups
        .into_iter()
        .enumerate()
        .flat_map(|(index, group)| {
            let action = if index < limit {
                QueueAction::Start
            } else {
                QueueAction::Queue
            };
            group.into_iter().map(move |record| (record.id, action))
        })
        .collect();
    planned
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::downloads::DownloadPriority;

    fn record(id: u64, priority: DownloadPriority, added_at: i64) -> DownloadRecord {
        DownloadRecord {
            id,
            anime_id: 1,
            provider: "anilist".into(),
            episode: Some(id as u32),
            title: "Show".into(),
            cover_image: None,
            episode_title: None,
            release_title: "release".into(),
            torrent_id: Some(id as usize),
            magnet: Some("magnet:?xt=urn:btih:test".into()),
            torrent_url: None,
            info_hash: None,
            cache_dir: None,
            file_idx: 0,
            file_name: format!("{id}.mkv"),
            status: DownloadStatus::Downloading,
            progress_bytes: 0,
            total_bytes: 100,
            release_seeders: None,
            output_path: String::new(),
            added_at,
            completed_at: None,
            priority,
            user_paused: false,
            season: None,
            library_dir: None,
            torrent_file_count: None,
            error: None,
        }
    }

    #[test]
    fn priority_then_age() {
        let records = vec![
            record(1, DownloadPriority::Low, 1),
            record(2, DownloadPriority::High, 3),
            record(3, DownloadPriority::Normal, 2),
        ];
        let plan = plan_queue(&records, 3);
        assert_eq!(
            plan.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            vec![2, 3, 1]
        );
    }

    #[test]
    fn limits_active_downloads() {
        let records = vec![
            record(1, DownloadPriority::Normal, 1),
            record(2, DownloadPriority::Normal, 2),
            record(3, DownloadPriority::Normal, 3),
        ];
        let plan = plan_queue(&records, 1);
        assert_eq!(plan[0], (1, QueueAction::Start));
        assert_eq!(plan[1], (2, QueueAction::Queue));
        assert_eq!(plan[2], (3, QueueAction::Queue));
    }

    #[test]
    fn user_paused_is_ignored() {
        let mut paused = record(1, DownloadPriority::High, 1);
        paused.user_paused = true;
        let records = vec![paused, record(2, DownloadPriority::Low, 2)];
        assert_eq!(plan_queue(&records, 1), vec![(2, QueueAction::Start)]);
    }

    #[test]
    fn shared_torrent_is_one_queue_unit() {
        let mut first = record(1, DownloadPriority::High, 1);
        let mut second = record(2, DownloadPriority::High, 1);
        second.torrent_id = first.torrent_id;
        let other = record(3, DownloadPriority::Low, 2);
        let plan = plan_queue(&[first.clone(), second.clone(), other], 1);
        assert_eq!(
            plan,
            vec![
                (1, QueueAction::Start),
                (2, QueueAction::Start),
                (3, QueueAction::Queue)
            ]
        );
        first.user_paused = true;
        assert_eq!(
            plan_queue(&[first, second], 1),
            vec![(2, QueueAction::Start)]
        );
    }

    #[test]
    fn a_stable_info_hash_groups_rows_with_different_stale_ids() {
        let mut first = record(1, DownloadPriority::High, 1);
        first.info_hash = Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into());
        let mut second = record(2, DownloadPriority::High, 1);
        second.info_hash = first.info_hash.clone();
        second.torrent_id = Some(99);
        let other = record(3, DownloadPriority::Low, 2);
        let plan = plan_queue(&[first, second, other], 1);
        assert_eq!(
            plan,
            vec![
                (1, QueueAction::Start),
                (2, QueueAction::Start),
                (3, QueueAction::Queue)
            ]
        );
    }
}
