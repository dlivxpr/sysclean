use sysclean::models::BackgroundTaskStatus;

#[test]
fn byte_progress_prefers_byte_ratio_and_label() {
    let mut status = BackgroundTaskStatus::new("缓存清理", "正在删除", true);
    status.bytes_current = Some(1_024);
    status.bytes_total = Some(2_048);
    status.progress_label = Some("已释放 1.0 KB / 2.0 KB".into());
    status.determinate = true;

    assert_eq!(status.progress_ratio(), Some(0.5));
    assert_eq!(
        status.progress_label_text(),
        Some("已释放 1.0 KB / 2.0 KB".to_string())
    );
}

#[test]
fn count_progress_falls_back_to_item_ratio() {
    let mut status = BackgroundTaskStatus::new("目录扫描", "正在计算大小", true);
    status.progress_current = 3;
    status.progress_total = 6;
    status.progress_label = Some("已完成 3/6".into());
    status.determinate = true;

    assert_eq!(status.progress_ratio(), Some(0.5));
    assert_eq!(status.progress_label_text(), Some("已完成 3/6".to_string()));
}

#[test]
fn indeterminate_status_does_not_report_progress_ratio() {
    let mut status = BackgroundTaskStatus::new("缓存清理", "正在删除 docker", true);
    status.progress_label = Some("正在清理 docker".into());
    status.determinate = false;

    assert_eq!(status.progress_ratio(), None);
    assert_eq!(
        status.progress_label_text(),
        Some("正在清理 docker".to_string())
    );
}
