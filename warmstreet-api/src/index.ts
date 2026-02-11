import { Hono } from 'hono';
import { logger } from 'hono/logger';
import { authMiddleware } from './middleware/auth';
import { dbContextMiddleware } from './middleware/db-context';
import { idempotencyMiddleware } from './middleware/idempotency';
import profileRoutes from './routes/profile';
import casesRoutes from './routes/cases';
import uploadsRoutes from './routes/uploads';
import aiRoutes from './routes/ai';
import { cleanupOrphanedMedia } from './lib/cleanup';
import { HonoEnv } from './types';

const app = new Hono<HonoEnv>();

app.use(logger());

// Global Middleware Chain (Strict Order)
app.use(authMiddleware);
app.use(dbContextMiddleware);
app.use(idempotencyMiddleware);

// Routes
app.route('/v1/profile', profileRoutes);
app.route('/v1/cases', casesRoutes);
app.route('/v1/cases', uploadsRoutes); // Media upload/download endpoints
app.route('/v1/ai', aiRoutes);         // Gemini AI analysis

// 404 for everything else
app.all('*', (c) => c.json({ error: 'Not Found' }, 404));

export default {
    fetch: app.fetch,

    // Scheduled cleanup cron (runs daily at 03:00 UTC)
    async scheduled(event: ScheduledEvent, env: any, ctx: ExecutionContext) {
        console.log('Starting scheduled media cleanup...');

        const mockContext = {
            env,
            executionCtx: ctx,
            get: (key: string) => env[key],
        } as any;

        const result = await cleanupOrphanedMedia(mockContext, 7);
        console.log(`Media cleanup complete: ${result.deleted} deleted, ${result.errors} errors`);
    }
};
