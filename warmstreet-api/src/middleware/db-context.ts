import { createMiddleware } from 'hono/factory';
import { getDbPool } from '../lib/db';
import { HonoEnv } from '../types';

export const dbContextMiddleware = createMiddleware<HonoEnv>(async (c, next) => {
    const user = c.get('user');
    if (!user) {
        return c.json({ error: 'Unauthorized: No user context found' }, 500);
    }

    const pool = getDbPool(c);
    c.set('pool', pool);

    // Using a single client for the transaction duration
    const client = await pool.connect();

    try {
        // Start transaction
        await client.query('BEGIN');

        // Set local variable for RLS/Auditing
        await client.query(`SELECT set_config('app.user_id', $1, true)`, [user.user_id]);

        c.set('db', client);

        await next();

        // Commit the transaction if the request was successful
        await client.query('COMMIT');
    } catch (err) {
        await client.query('ROLLBACK');
        throw err;
    } finally {
        client.release();
    }
});
