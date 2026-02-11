import { Pool, PoolClient } from '@neondatabase/serverless';
import { Context } from 'hono';
import { HonoEnv } from '../types';

export const getDbPool = (c: Context<HonoEnv>): Pool => {
    return new Pool({
        connectionString: c.env.NEON_DATABASE_URL,
    });
};

export async function execute_txn<T>(
    pool: Pool,
    userId: string | null,
    callback: (client: PoolClient) => Promise<T>
): Promise<T> {
    const client = await pool.connect();
    try {
        await client.query('BEGIN');

        // Set RLS context for Row-Level Security
        if (userId) {
            await client.query("SELECT set_config('app.user_id', $1, true)", [userId]);
        }

        const result = await callback(client);
        await client.query('COMMIT');
        return result;
    } catch (e) {
        await client.query('ROLLBACK');
        throw e;
    } finally {
        client.release();
    }
}
