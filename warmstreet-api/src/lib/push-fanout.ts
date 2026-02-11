import { Pool } from '@neondatabase/serverless';
import { FCMClient, FCMPayload } from './fcm';

export async function fanoutNewCase(
    db: Pool,
    fcm: FCMClient,
    caseId: string,
    caseLocation: { lat: number; lng: number },
    reporterId: string,
    description: string,
    ctx: any // ExecutionContext
) {
    ctx.waitUntil((async () => {
        const volunteers = await db.query(
            `SELECT user_id, fcm_token
             FROM public.get_nearby_volunteers(
                 ST_MakePoint($1, $2)::geography, 10000
             )`,
            [caseLocation.lng, caseLocation.lat]
        );

        for (const v of volunteers.rows) {
            if (v.user_id === reporterId) continue;
            if (!v.fcm_token) continue;

            // Dedupe check
            const exists = await db.query(
                `SELECT 1 FROM public.case_notifications_sent
                 WHERE case_id = $1 AND user_id = $2 AND kind = 'new_rescue'`,
                [caseId, v.user_id]
            );
            if (exists.rows.length > 0) continue;

            const payload: FCMPayload = {
                data: {
                    case_id: caseId,
                    type: 'new_rescue',
                    lat: String(caseLocation.lat),
                    lng: String(caseLocation.lng),
                },
                notification: {
                    title: 'Animal needs help nearby',
                    body: description?.substring(0, 100) || 'A rescue case was reported near you',
                },
                android: {
                    priority: 'high',
                },
                apns: {
                    payload: {
                        aps: {
                            contentAvailable: true,
                            sound: 'default',
                        },
                    },
                },
            };

            const result = await fcm.sendWithCleanup(v.fcm_token, v.user_id, payload, db);

            if (result.success) {
                await db.query(
                    `INSERT INTO public.case_notifications_sent (case_id, user_id, kind)
                     VALUES ($1, $2, 'new_rescue')
                     ON CONFLICT DO NOTHING`,
                    [caseId, v.user_id]
                );
            }
        }
    })());
}
