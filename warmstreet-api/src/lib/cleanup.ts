import { Pool } from '@neondatabase/serverless';
import { getTigrisClient, deleteObject } from './tigris';
import { Context } from 'hono';
import { HonoEnv } from '../types';

/**
 * Cleanup resolved/cancelled/unreachable cases older than specified days
 * Deletes Tigris objects and clears DB object keys
 * 
 * Run daily via Cloudflare Worker cron trigger
 */
export async function cleanupOrphanedMedia(
    c: Context<HonoEnv>,
    daysOld: number = 7
): Promise<{ deleted: number; errors: number }> {
    const db = new Pool({ connectionString: c.env.NEON_DATABASE_URL });
    const tigris = getTigrisClient(c);
    const bucket = c.env.TIGRIS_BUCKET || 'warmstreet-production';

    let deleted = 0;
    let errors = 0;

    try {
        // Query cases eligible for cleanup
        const result = await db.query(`
            SELECT id, photo_object_key, crop_object_key
            FROM public.rescue_cases
            WHERE status IN ('resolved', 'cancelled', 'unreachable')
              AND resolved_at IS NOT NULL
              AND resolved_at < now() - INTERVAL '${daysOld} days'
              AND (photo_object_key IS NOT NULL OR crop_object_key IS NOT NULL)
            LIMIT 100
        `);

        const cases = result.rows as Array<{
            id: string;
            photo_object_key: string | null;
            crop_object_key: string | null;
        }>;

        console.log(`Found ${cases.length} cases eligible for media cleanup`);

        // Delete objects from Tigris and clear DB keys
        for (const caseItem of cases) {
            try {
                // Delete original photo if exists
                if (caseItem.photo_object_key) {
                    await deleteObject(tigris, bucket, caseItem.photo_object_key);
                }

                // Delete crop if exists
                if (caseItem.crop_object_key) {
                    await deleteObject(tigris, bucket, caseItem.crop_object_key);
                }

                // Clear keys in DB
                await db.query(
                    'SELECT cleanup_media_keys($1)',
                    [caseItem.id]
                );

                deleted++;
            } catch (err) {
                console.error(`Failed to cleanup case ${caseItem.id}:`, err);
                errors++;
            }
        }

        return { deleted, errors };
    } finally {
        await db.end();
    }
}

/**
 * Immediate cleanup on case resolution
 * Called from transition endpoint when case reaches terminal status
 */
export async function cleanupCaseMedia(
    c: Context<HonoEnv>,
    caseId: string
): Promise<void> {
    const db = new Pool({ connectionString: c.env.NEON_DATABASE_URL });
    const tigris = getTigrisClient(c);
    const bucket = c.env.TIGRIS_BUCKET || 'warmstreet-production';

    try {
        // Get object keys
        const result = await db.query(
            'SELECT photo_object_key, crop_object_key FROM public.rescue_cases WHERE id = $1',
            [caseId]
        );

        if (result.rows.length === 0) return;

        const caseData = result.rows[0] as {
            photo_object_key: string | null;
            crop_object_key: string | null;
        };

        // Delete objects (best-effort)
        const deletions = [];
        if (caseData.photo_object_key) {
            deletions.push(deleteObject(tigris, bucket, caseData.photo_object_key));
        }
        if (caseData.crop_object_key) {
            deletions.push(deleteObject(tigris, bucket, caseData.crop_object_key));
        }

        await Promise.allSettled(deletions);

        // Clear keys in DB
        await db.query('SELECT cleanup_media_keys($1)', [caseId]);
    } catch (err) {
        console.error(`Failed to cleanup media for case ${caseId}:`, err);
        // Don't throw - cleanup is best-effort
    } finally {
        await db.end();
    }
}
