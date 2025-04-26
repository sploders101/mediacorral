SELECT
    id,
    video_type,
    match_id,
    resolution_width,
    resolution_height,
    length,
    rip_job,
    HEX(duplicates.original_video_hash)
FROM (
    SELECT
        COUNT() AS count,
        original_video_hash
    FROM video_files
    GROUP BY
        original_video_hash
) duplicates
JOIN video_files ON
    duplicates.original_video_hash = video_files.original_video_hash
WHERE
    count > 1
ORDER BY
    duplicates.original_video_hash;
