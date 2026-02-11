import { createMiddleware } from 'hono/factory';
import { createRemoteJWKSet, jwtVerify } from 'jose';
import { HonoEnv } from '../types';

// Simple in-memory cache for JWKS (reloaded on cold start, effective for worker lifetime)
let JWKS: ReturnType<typeof createRemoteJWKSet> | null = null;

export const authMiddleware = createMiddleware<HonoEnv>(async (c, next) => {
    const authHeader = c.req.header('Authorization');
    if (!authHeader || !authHeader.startsWith('Bearer ')) {
        return c.json({ error: 'Unauthorized' }, 401);
    }

    const token = authHeader.split(' ')[1];

    try {
        if (!JWKS) {
            JWKS = createRemoteJWKSet(new URL(c.env.NEON_AUTH_JWKS_URL));
        }

        const { payload } = await jwtVerify(token, JWKS, {
            algorithms: ['RS256'], // Standard for many providers, verify if Neon uses others
        });

        if (!payload.sub) {
            throw new Error('No subject in JWT');
        }

        c.set('user', { user_id: payload.sub });
        c.set('userId', payload.sub);
        await next();
    } catch (err) {
        console.error('Auth verification failed:', err);
        return c.json({ error: 'Unauthorized' }, 401);
    }
});
