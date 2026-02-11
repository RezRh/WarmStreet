import { PoolClient } from '@neondatabase/serverless';

export interface IdempotencyRecord {
    status: number;
    headers: Record<string, string>;
    body: any; // We store minimal processed body if needed, or null if body is not critical
}

export async function getCachedResponse(
    db: PoolClient,
    userId: string,
    path: string,
    key: string
): Promise<IdempotencyRecord | null> {
    const res = await db.query(
        'SELECT response FROM public.idempotency_log WHERE user_id = $1 AND endpoint = $2 AND key = $3',
        [userId, path, key]
    );

    if (res.rows.length > 0) {
        return res.rows[0].response as IdempotencyRecord;
    }
    return null;
}

export async function cacheResponse(
    db: PoolClient,
    userId: string,
    path: string,
    key: string,
    status: number,
    headers: Record<string, string>,
    body: any
): Promise<void> {
    // Only store body if it's small/critical. For large responses, we might want to store a reference or just success status.
    // For now, we store it but the middleware will be careful about what 'body' it passes.
    const record: IdempotencyRecord = { status, headers, body };

    await db.query(
        'INSERT INTO public.idempotency_log (user_id, endpoint, key, response) VALUES ($1, $2, $3, $4)',
        [userId, path, key, JSON.stringify(record)]
    );
}
