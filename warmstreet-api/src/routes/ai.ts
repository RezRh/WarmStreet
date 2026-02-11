import { Hono } from 'hono';
import { HonoEnv } from '../types';
import { GeminiClient } from '../lib/gemini';
import { getTigrisClient } from '../lib/tigris';
import { GetObjectCommand } from '@aws-sdk/client-s3';

const app = new Hono<HonoEnv>();

/**
 * POST /v1/ai/gemini/:id/analyze
 * Trigger Gemini AI analysis for a case
 */
app.post('/:id/analyze', async (c) => {
    const caseId = c.req.param('id');
    const userId = c.get('userId');
    const db = c.get('db'); // PoolClient

    if (!userId || !db) return c.json({ error: 'Unauthorized' }, 401);

    try {
        // 1. Get object key and verify access (via RLS)
        const res = await db.query(
            'SELECT crop_object_key, photo_object_key, description FROM public.rescue_cases WHERE id = $1',
            [caseId]
        );
        const caseData = res.rows[0];

        if (!caseData) return c.json({ error: 'Case not found or access denied' }, 404);

        const objectKey = caseData.crop_object_key || caseData.photo_object_key;
        if (!objectKey) {
            return c.json({ error: 'No media available for analysis' }, 400);
        }

        // 2. Fetch image data from Tigris
        const tigris = getTigrisClient(c);
        const bucket = c.env.TIGRIS_BUCKET || 'warmstreet-production';

        const getCmd = new GetObjectCommand({
            Bucket: bucket,
            Key: objectKey
        });

        const { Body } = await tigris.send(getCmd);
        if (!Body) throw new Error('Failed to fetch image from Tigris');

        const imageBytes = await Body.transformToUint8Array();

        // 3. Analyze with Gemini
        const gemini = new GeminiClient(c.env.GEMINI_API_KEY);
        const diagnosis = await gemini.analyzeWound(imageBytes, caseData.description);

        // 4. Update DB
        await db.query(
            'UPDATE public.rescue_cases SET gemini_diagnosis = $1, updated_at = now() WHERE id = $2',
            [JSON.stringify(diagnosis), caseId]
        );

        return c.json({ success: true, diagnosis });

    } catch (e: any) {
        console.error('Gemini analysis error:', e);
        return c.json({ error: 'Internal Server Error' }, 500);
    }
});

export default app;
