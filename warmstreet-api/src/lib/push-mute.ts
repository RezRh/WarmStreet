import { Pool } from '@neondatabase/serverless';
import { FCMClient, FCMPayload } from './fcm';

export async function muteOnClaim(
    db: Pool,
    fcm: FCMClient,
    caseId: string,
    claimerId: string,
    ctx: any // ExecutionContext
) {
    ctx.waitUntil((async () => {
        // Find all users who were notified about this case, excluding the claimer
        const targets = await db.query(
            `SELECT DISTINCT n.user_id, p.fcm_token
             FROM public.case_notifications_sent n
             JOIN public.user_profile p ON p.user_id = n.user_id
             WHERE n.case_id = $1
               AND n.kind = 'new_rescue'
               AND n.user_id != $2
               AND p.fcm_token IS NOT NULL`,
            [caseId, claimerId]
        );

        for (const t of targets.rows) {
            // Check if already muted
            const exists = await db.query(
                `SELECT 1 FROM public.case_notifications_sent
                 WHERE case_id = $1 AND user_id = $2 AND kind = 'mute'`,
                [caseId, t.user_id]
            );
            if (exists.rows.length > 0) continue;

            // Silent push — data only, NO notification field
            const payload: FCMPayload = {
                data: {
                    case_id: caseId,
                    type: 'mute',
                    claimed_by: claimerId,
                },
                android: {
                    priority: 'high',
                },
                apns: {
                    payload: {
                        aps: {
                            contentAvailable: true,
                        },
                    },
                },
            };

            const result = await fcm.sendWithCleanup(t.fcm_token, t.user_id, payload, db);

            if (result.success) {
                await db.query(
                    `INSERT INTO public.case_notifications_sent (case_id, user_id, kind)
                     VALUES ($1, $2, 'mute')
                     ON CONFLICT DO NOTHING`,
                    [caseId, t.user_id]
                );
            }
        }
    })());
}
