import jwt from '@tsndr/cloudflare-worker-jwt';
import { Pool } from '@neondatabase/serverless';

interface ServiceAccount {
    client_email: string;
    private_key: string;
    token_uri: string;
    project_id: string;
}

export interface FCMPayload {
    data?: Record<string, string>;
    notification?: {
        title: string;
        body: string;
    };
    android?: {
        priority: 'high' | 'normal';
    };
    apns?: {
        payload: {
            aps: {
                contentAvailable?: boolean;
                sound?: string;
            };
        };
    };
}

export class FCMClient {
    private sa: ServiceAccount;
    private cachedToken: string | null = null;
    private tokenExpiry: number = 0;

    constructor(serviceAccountJson: string) {
        this.sa = JSON.parse(serviceAccountJson);
    }

    async getAccessToken(): Promise<string> {
        const now = Math.floor(Date.now() / 1000);
        if (this.cachedToken && now < this.tokenExpiry) {
            return this.cachedToken;
        }

        const assertion = await jwt.sign({
            iss: this.sa.client_email,
            scope: 'https://www.googleapis.com/auth/firebase.messaging',
            aud: this.sa.token_uri,
            iat: now,
            exp: now + 3600,
        }, this.sa.private_key, { algorithm: 'RS256' });

        const resp = await fetch(this.sa.token_uri, {
            method: 'POST',
            headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
            body: `grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&assertion=${assertion}`,
        });

        if (!resp.ok) {
            throw new Error(`OAuth2 token exchange failed: ${resp.status}`);
        }

        const data = await resp.json() as { access_token: string; expires_in?: number };
        this.cachedToken = data.access_token;
        this.tokenExpiry = now + 3300; // 55 min cache
        return this.cachedToken;
    }

    async send(fcmToken: string, payload: FCMPayload): Promise<boolean> {
        const accessToken = await this.getAccessToken();
        const resp = await fetch(
            `https://fcm.googleapis.com/v1/projects/${this.sa.project_id}/messages:send`,
            {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${accessToken}`,
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    message: {
                        token: fcmToken,
                        ...payload,
                    },
                }),
            }
        );
        return resp.ok;
    }

    async sendWithCleanup(
        fcmToken: string,
        userId: string,
        payload: FCMPayload,
        db: Pool
    ): Promise<{ success: boolean; reason?: string }> {
        const accessToken = await this.getAccessToken();
        const resp = await fetch(
            `https://fcm.googleapis.com/v1/projects/${this.sa.project_id}/messages:send`,
            {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${accessToken}`,
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    message: {
                        token: fcmToken,
                        ...payload,
                    },
                }),
            }
        );

        if (!resp.ok) {
            const err = await resp.json() as any;
            const errorCode = err.error?.details?.[0]?.errorCode || err.error?.status;

            // Cleanup invalid tokens
            if (errorCode === 'UNREGISTERED' || resp.status === 404) {
                console.log(`Cleaning up invalid token for user ${userId}`);
                await db.query(
                    'UPDATE public.user_profile SET fcm_token = NULL WHERE user_id = $1',
                    [userId]
                );
                return { success: false, reason: 'unregistered' };
            }
            return { success: false, reason: 'error' };
        }

        return { success: true };
    }
}
