SELECT
    save_code,
    created_at
FROM
    backups
ORDER BY
    id DESC
LIMIT
    1;