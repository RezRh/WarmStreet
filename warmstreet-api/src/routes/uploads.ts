import { Hono } from 'hono';
import { HonoEnv } from '../types';
import { getTigrisClient, generateSignedPutUrl, generateSignedGetUrl } from '../lib/tigris';

const app = new Hono<HonoEnv>();

/**
 * POST /v1/cases/:id/uploads
 * Generate signed PUT URLs for uploading case media (original + crop)
 * 
 * Security: RLS ensures user can only get URLs for cases they can access
 */
app.post('/:id/uploads', async (c) => {
    const caseId = c.req.param('id');
    const userId = c.get('userId');
    const sql = c.get('sql');

    if (!userId) return c.json({ error: 'Unauthorized' }, 401);

    try {
        // Verify user has access to this case (via RLS)
        const [caseExists] = await sql`
            SELECT id FROM public.rescue_cases 
            WHERE id = ${caseId}
            LIMIT 1
        `;

        if (!caseExists) {
            return c.json({ error: 'Case not found or access denied' }, 404);
        }

        const tigris = getTigrisClient(c);
        const bucket = c.env.TIGRIS_BUCKET || 'warmstreet-production';

        // Generate object keys
        const originalKey = `rescues/${caseId}/original.webp`;
        const cropKey = `rescues/${caseId}/wound-crop.webp`;

        // Generate signed PUT URLs (15 minute expiry)
        const [originalPutUrl, cropPutUrl] = await Promise.all([
            generateSignedPutUrl(tigris, bucket, originalKey, 900),
            generateSignedPutUrl(tigris, bucket, cropKey, 900),
        ]);

        return c.json({
            original_put_url: originalPutUrl,
            crop_put_url: cropPutUrl,
            object_keys: {
                photo_object_key: originalKey,
                crop_object_key: cropKey,
            },
            expires_in_seconds: 900,
        });
    } catch (e: any) {
        console.error('Uploads error:', e);
        return c.json({ error: 'Internal Server Error' }, 500);
    }
});

/**
 * GET /v1/cases/:id/media?kind=original|crop
 * Generate signed GET URL for viewing case media
 * 
 * Security: RLS ensures user can only get URLs for cases they can access
 */
app.get('/:id/media', async (c) => {
    const caseId = c.req.param('id');
    const kind = c.req.query('kind'); // 'original' or 'crop'
    const userId = c.get('userId');
    const sql = c.get('sql');

    if (!userId) return c.json({ error: 'Unauthorized' }, 401);
    if (!kind || !['original', 'crop'].includes(kind)) {
        return c.json({ error: 'Invalid kind parameter. Must be "original" or "crop".' }, 400);
    }

    try {
        // Fetch case to verify access + get object key (via RLS)
        const [caseData] = await sql`
            SELECT photo_object_key, crop_object_key 
            FROM public.rescue_cases 
            WHERE id = ${caseId}
            LIMIT 1
        `;

        if (!caseData) {
            return c.json({ error: 'Case not found or access denied' }, 404);
        }

        const objectKey = kind === 'original' ? caseData.photo_object_key : caseData.crop_object_key;

        if (!objectKey) {
            return c.json({ error: `No ${kind} media available for this case` }, 404);
        }

        const tigris = getTigrisClient(c);
        const bucket = c.env.TIGRIS_BUCKET || 'warmstreet-production';

        // Generate signed GET URL (5 minute expiry)
        const signedUrl = await generateSignedGetUrl(tigris, bucket, objectKey, 300);

        return c.json({
            url: signedUrl,
            object_key: objectKey,
            expires_in_seconds: 300,
        });
    } catch (e: any) {
        console.error('Media retrieval error:', e);
        return c.json({ error: 'Internal Server Error' }, 500);
    }
});

/**
 * PATCH /v1/cases/:id/media-keys
 * Update case with uploaded media object keys
 * Called after client successfully uploads via signed PUT URLs
 */
app.patch('/:id/media-keys', async (c) => {
    const caseId = c.req.param('id');
    const userId = c.get('userId');
    const sql = c.get('sql');

    if (!userId) return c.json({ error: 'Unauthorized' }, 401);

    const body = await c.req.json() as {
        photo_object_key?: string;
        crop_object_key?: string;
    };

    try {
        // Update object keys (RLS ensures only reporter can update)
        const [updated] = await sql`
            UPDATE public.rescue_cases
            SET photo_object_key = COALESCE(${body.photo_object_key}, photo_object_key),
                crop_object_key = COALESCE(${body.crop_object_key}, crop_object_key),
                updated_at = now()
            WHERE id = ${caseId} AND reporter_id = ${userId}
            RETURNING id
        `;

        if (!updated) {
            return c.json({ error: 'Case not found or unauthorized' }, 404);
        }

        return c.json({ success: true });
    } catch (e: any) {
        console.error('Media keys update error:', e);
        return c.json({ error: 'Internal Server Error' }, 500);
    }
});

export default app;
