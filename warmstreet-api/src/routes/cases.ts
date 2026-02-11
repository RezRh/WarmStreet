import { Hono } from 'hono';
import { HonoEnv } from '../types';
import { FCMClient } from '../lib/fcm';
import { fanoutNewCase } from '../lib/push-fanout';
import { muteOnClaim } from '../lib/push-mute';

const app = new Hono<HonoEnv>();

// 1. POST /v1/cases (Create Case)
app.post('/', async (c) => {
    const userId = c.get('userId');
    const db = c.get('db'); // PoolClient

    if (!userId || !db) return c.json({ error: 'Unauthorized or DB context missing' }, 401);

    const body = await c.req.json() as {
        location: [number, number],
        description?: string,
        wound_severity?: number,
        landmark_hint?: string
    };

    try {
        // 1. Create the case
        const res = await db.query(
            `INSERT INTO public.rescue_cases (reporter_id, location, description, wound_severity, landmark_hint)
             VALUES ($1, ST_SetSRID(ST_MakePoint($2, $3), 4326), $4, $5, $6)
             RETURNING id, description, wound_severity`,
            [userId, body.location[1], body.location[0], body.description, body.wound_severity, body.landmark_hint]
        );

        const newCase = res.rows[0];

        if (!newCase) throw new Error('Failed to create case');

        // 2. Queue fanout (async)
        // We offload the heavy fanout logic to a Consumer Worker via Cloudflare Queues.
        // This prevents the API request from timing out or hitting memory limits.
        try {
            await c.env.QUEUE.send({
                type: 'fanout_new_case',
                caseId: newCase.id,
                location: { lat: body.location[0], lng: body.location[1] },
                userId,
                description: body.description || ''
            });
        } catch (e) {
            console.error('Failed to queue fanout:', e);
            // We don't fail the request, but we should log it.
            // Ideally we'd have a fallback or retry mechanism.
        }

        return c.json({ success: true, id: newCase.id });

    } catch (e: any) {
        console.error('Create case error:', e);
        return c.json({ error: 'Internal Server Error' }, 500);
    }
});

// 2. POST /v1/cases/:id/claim
app.post('/:id/claim', async (c) => {
    const caseId = c.req.param('id');
    const userId = c.get('userId');
    const db = c.get('db');

    if (!userId || !db) return c.json({ error: 'Unauthorized' }, 401);

    try {
        const res = await db.query('SELECT claim_rescue_case($1, $2) as success', [caseId, userId]);
        const result = res.rows[0];

        if (result && result.success) {
            // Queue mute fanout (async)
            try {
                await c.env.QUEUE.send({
                    type: 'fanout_case_claim',
                    caseId,
                    userId
                });
            } catch (e) {
                console.error('Failed to queue mute fanout:', e);
            }

            return c.json({ claimed: true, status: 'claimed' });
        } else {
            const currentRes = await db.query(
                'SELECT status, assigned_rescuer_id FROM public.rescue_cases WHERE id = $1',
                [caseId]
            );
            const current = currentRes.rows[0];
            return c.json({ claimed: false, status: current?.status, assigned_rescuer_id: current?.assigned_rescuer_id }, 409);
        }
    } catch (e: any) {
        console.error('Claim error:', e);
        return c.json({ error: 'Internal Server Error' }, 500);
    }
});

// 3. POST /v1/cases/:id/transition
app.post('/:id/transition', async (c) => {
    const caseId = c.req.param('id');
    const userId = c.get('userId');
    const db = c.get('db');

    if (!userId || !db) return c.json({ error: 'Unauthorized' }, 401);

    const { next_status } = await c.req.json() as { next_status: string };

    try {
        const res = await db.query('SELECT transition_case($1, $2, $3) as ok', [caseId, userId, next_status]);
        const result = res.rows[0];

        if (result && result.ok) {
            if (['resolved', 'cancelled', 'unreachable'].includes(next_status)) {
                // Background immediate cleanup: Delete from Tigris THEN clear DB keys
                const cleanup = import('../lib/cleanup').then(m => m.cleanupCaseMedia(c, caseId));
                c.executionCtx.waitUntil(cleanup);

                // Notify Reporter
                c.executionCtx.waitUntil((async () => {
                    const reporterRes = await db.query(`
                        SELECT up.user_id, up.fcm_token 
                        FROM public.rescue_cases rc
                        JOIN public.user_profile up ON rc.reporter_id = up.user_id
                        WHERE rc.id = $1 AND up.fcm_token IS NOT NULL
                     `, [caseId]);

                    const reporter = reporterRes.rows[0];

                    if (reporter && reporter.user_id !== userId) {
                        const serviceAccountJson = JSON.stringify({
                            client_email: c.env.FIREBASE_CLIENT_EMAIL,
                            private_key: c.env.FIREBASE_PRIVATE_KEY.replace(/\\n/g, '\n'),
                            token_uri: 'https://oauth2.googleapis.com/token',
                            project_id: c.env.FIREBASE_PROJECT_ID,
                        });
                        const fcm = new FCMClient(serviceAccountJson);

                        const payload = {
                            data: {
                                case_id: caseId,
                                type: 'case_update',
                                new_status: next_status,
                            },
                            notification: {
                                title: 'Rescue Update',
                                body: `The rescue you reported is now ${next_status.replace('_', ' ')}.`,
                            },
                            android: { priority: 'high' as const },
                            apns: {
                                payload: {
                                    aps: {
                                        contentAvailable: true,
                                        sound: 'default',
                                    },
                                },
                            },
                        };

                        await fcm.sendWithCleanup(reporter.fcm_token, reporter.user_id, payload, db as any); // FCM wants Pool but Client might work?
                    }
                })());
            }
            return c.json({ ok: true, status: next_status });
        } else {
            const currentRes = await db.query('SELECT status FROM public.rescue_cases WHERE id = $1', [caseId]);
            const current = currentRes.rows[0];
            return c.json({ ok: false, status: current?.status }, 409);
        }
    } catch (e: any) {
        console.error('Transition error:', e);
        return c.json({ error: 'Internal Server Error' }, 500);
    }
});

export default app;
