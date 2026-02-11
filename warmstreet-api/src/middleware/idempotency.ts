import { createMiddleware } from 'hono/factory';
import { HonoEnv } from '../types';
import { getCachedResponse, cacheResponse } from '../lib/idempotency';

export const idempotencyMiddleware = createMiddleware<HonoEnv>(async (c, next) => {
    const user = c.get('user');
    const db = c.get('db');

    if (!user || !db) {
        return await next();
    }

    if (c.req.method === 'GET' || c.req.method === 'HEAD') {
        return await next();
    }

    const key = c.req.header('Idempotency-Key');
    if (!key) {
        return c.json({ error: 'Missing Idempotency-Key header' }, 400);
    }

    const path = c.req.path;

    // 1. Check log
    const cached = await getCachedResponse(db, user.user_id, path, key);

    if (cached) {
        // Return cached response
        // DO NOT COMMIT here. Let the parent dbContextMiddleware handle the transaction lifecycle.
        // We just return the response, and the chain unwinds.
        return c.json(cached.body, cached.status as any, cached.headers);
    }

    // 2. Proceed
    await next();

    // 3. Store response
    const res = c.res;

    // Only cache successful or client-error responses
    if (res.status >= 500) {
        return;
    }

    // AVOID res.clone().json() for large bodies if possible.
    // Ideally, we should capture the body from the handler return, but Hono abstraction makes c.res generic.
    // We will attempt to clone but with a size limit or catch. 
    // BETTER: The handler should ideally return a JSON object that we can intercept? 
    // Hono middleware next() runs, and c.res is set.

    try {
        let bodyToStore = {};

        // Only try to parse JSON if content-type is json
        const contentType = res.headers.get('content-type');
        if (contentType && contentType.includes('application/json')) {
            // We have to clone to read it without consuming the stream for the client
            const clonedRes = res.clone();
            bodyToStore = await clonedRes.json();
        }

        // Extract headers
        const headers: Record<string, string> = {};
        res.headers.forEach((v, k) => {
            headers[k] = v;
        });

        await cacheResponse(db, user.user_id, path, key, res.status, headers, bodyToStore);
    } catch (e) {
        console.warn('Failed to cache idempotency response', e);
        // Do not fail the request if caching fails, unless it's a transaction constraint?
        // If INSERT fails (e.g. duplicate key race condition), it throws.
        // If it throws, dbContext will ROLLBACK the whole thing, which is CORRECT for consistency.
        // So we let it throw.
        throw e;
    }
});
