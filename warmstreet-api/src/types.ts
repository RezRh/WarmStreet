import { Pool, PoolClient } from '@neondatabase/serverless';

export type Bindings = {
    NEON_DATABASE_URL: string;
    NEON_AUTH_JWKS_URL: string;
    TIGRIS_ACCESS_KEY_ID: string;
    TIGRIS_SECRET_ACCESS_KEY: string;
    TIGRIS_BUCKET: string;
    TIGRIS_ENDPOINT: string;
    FIREBASE_PROJECT_ID: string;
    FIREBASE_CLIENT_EMAIL: string;
    FIREBASE_PRIVATE_KEY: string;
    GOOGLE_SERVICE_ACCOUNT_JSON: string;
    GEMINI_API_KEY: string;
    QUEUE: Queue;
};

export type Variables = {
    user?: {
        user_id: string;
    };
    userId?: string;  // Direct user ID access
    db?: PoolClient;  // MUST be PoolClient since db-context sets a transaction-scoped client
    sql?: any;        // sql tag helper (neon function)
    pool?: Pool;      // Global DB pool if needed
};

export type HonoEnv = {
    Bindings: Bindings;
    Variables: Variables;
};
