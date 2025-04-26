DELETE FROM match_info
WHERE
    match_info.id IN (
        SELECT match_info.id
        FROM match_info
        JOIN video_files ON video_files.id = match_info.video_file_id
        JOIN tv_episodes ON video_files.match_id = tv_episodes.id
        WHERE
            video_files.video_type = 3
            AND tv_episodes.id IS NOT NULL
        ORDER BY match_id
    );
