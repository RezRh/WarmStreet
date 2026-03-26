#!/usr/bin/env node

/**
 * Appwrite Database Setup Script for WarmStreet Messaging
 * 
 * This script creates the required database, collections, and indexes
 * for the E2E encrypted messaging feature.
 * 
 * Usage:
 *   node setup-appwrite-messaging.js
 * 
 * Requirements:
 *   - Appwrite project ID: 69b47d6d002eda226e97
 *   - Admin API key (set APPWRITE_API_KEY environment variable)
 */

import { Client, Databases, ID, Permission, Role } from 'node-appwrite';

// Configuration
const APPWRITE_ENDPOINT = 'https://sgp.cloud.appwrite.io/v1';
const APPWRITE_PROJECT_ID = '69b47d6d002eda226e97';
const APPWRITE_API_KEY = process.env.APPWRITE_API_KEY;

// Database configuration
const DATABASE_ID = 'warmstreet_messaging';
const DATABASE_NAME = 'WarmStreet Messaging';

// Collection IDs
const CONVERSATIONS_COLLECTION_ID = 'conversations';
const MESSAGES_COLLECTION_ID = 'messages';

// Check for API key
if (!APPWRITE_API_KEY) {
  console.error('❌ Error: APPWRITE_API_KEY environment variable not set');
  console.error('');
  console.error('To get your API key:');
  console.error('1. Go to https://cloud.appwrite.io');
  console.error('2. Select your project (WarmStreet)');
  console.error('3. Go to API Keys in the left sidebar');
  console.error('4. Create a new API key with these scopes:');
  console.error('   - databases.read');
  console.error('   - databases.write');
  console.error('   - collections.read');
  console.error('   - collections.write');
  console.error('   - documents.read');
  console.error('   - documents.write');
  console.error('5. Copy the key and run:');
  console.error('   export APPWRITE_API_KEY="your-key-here"');
  console.error('');
  process.exit(1);
}

// Initialize client
const client = new Client()
  .setEndpoint(APPWRITE_ENDPOINT)
  .setProject(APPWRITE_PROJECT_ID)
  .setKey(APPWRITE_API_KEY);

const databases = new Databases(client);

// Helper functions
async function createDatabase() {
  try {
    console.log('📦 Creating database...');
    const database = await databases.create(DATABASE_ID, DATABASE_NAME);
    console.log('✅ Database created:', database.name);
    return database;
  } catch (error) {
    if (error.code === 409) {
      console.log('ℹ️  Database already exists, skipping...');
      return null;
    }
    throw error;
  }
}

async function createConversationsCollection() {
  try {
    console.log('\n💬 Creating conversations collection...');
    
    const collection = await databases.createCollection(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      'Conversations',
      [
        Permission.read(Role.users()),
        Permission.create(Role.users()),
        Permission.update(Role.users()),
        Permission.delete(Role.users())
      ]
    );
    
    console.log('✅ Collection created:', collection.name);
    
    // Create attributes
    console.log('  → Creating attributes...');
    
    // participant_ids (array of strings)
    await databases.createStringAttribute(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      'participant_ids',
      100,
      false,
      undefined,
      true, // Array
      [Permission.read(Role.users()), Permission.update(Role.users())]
    );
    console.log('     ✓ participant_ids');
    
    // participant_details (long text for JSON)
    await databases.createStringAttribute(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      'participant_details',
      10000,
      false,
      undefined,
      false,
      [Permission.read(Role.users()), Permission.update(Role.users())]
    );
    console.log('     ✓ participant_details');
    
    // last_message (long text for JSON)
    await databases.createStringAttribute(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      'last_message',
      10000,
      false,
      undefined,
      false,
      [Permission.read(Role.users()), Permission.update(Role.users())]
    );
    console.log('     ✓ last_message');
    
    // unread_count (integer)
    await databases.createIntegerAttribute(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      'unread_count',
      false,
      0,
      undefined,
      undefined,
      [Permission.read(Role.users()), Permission.update(Role.users())]
    );
    console.log('     ✓ unread_count');
    
    // Create indexes
    console.log('  → Creating indexes...');
    
    await databases.createIndex(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      'participant_ids_idx',
      'key',
      ['participant_ids'],
      ['ASC']
    );
    console.log('     ✓ participant_ids_idx');
    
    await databases.createIndex(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      'updated_at_idx',
      'key',
      ['$updatedAt'],
      ['DESC']
    );
    console.log('     ✓ updated_at_idx');
    
    return collection;
  } catch (error) {
    if (error.code === 409) {
      console.log('ℹ️  Conversations collection already exists, skipping...');
      return null;
    }
    throw error;
  }
}

async function createMessagesCollection() {
  try {
    console.log('\n📨 Creating messages collection...');
    
    const collection = await databases.createCollection(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      'Messages',
      [
        Permission.read(Role.users()),
        Permission.create(Role.users()),
        Permission.update(Role.users()),
        Permission.delete(Role.users())
      ]
    );
    
    console.log('✅ Collection created:', collection.name);
    
    // Create attributes
    console.log('  → Creating attributes...');
    
    // conversation_id (string)
    await databases.createStringAttribute(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      'conversation_id',
      100,
      true,
      undefined,
      false,
      [Permission.read(Role.users()), Permission.create(Role.users())]
    );
    console.log('     ✓ conversation_id');
    
    // sender_id (string)
    await databases.createStringAttribute(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      'sender_id',
      100,
      true,
      undefined,
      false,
      [Permission.read(Role.users()), Permission.create(Role.users())]
    );
    console.log('     ✓ sender_id');
    
    // receiver_id (string)
    await databases.createStringAttribute(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      'receiver_id',
      100,
      true,
      undefined,
      false,
      [Permission.read(Role.users()), Permission.create(Role.users())]
    );
    console.log('     ✓ receiver_id');
    
    // encrypted_content (long text)
    await databases.createStringAttribute(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      'encrypted_content',
      100000,
      true,
      undefined,
      false,
      [Permission.read(Role.users()), Permission.create(Role.users())]
    );
    console.log('     ✓ encrypted_content');
    
    // nonce (string)
    await databases.createStringAttribute(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      'nonce',
      100,
      true,
      undefined,
      false,
      [Permission.read(Role.users()), Permission.create(Role.users())]
    );
    console.log('     ✓ nonce');
    
    // read (boolean)
    await databases.createBooleanAttribute(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      'read',
      false,
      false,
      [Permission.read(Role.users()), Permission.create(Role.users()), Permission.update(Role.users())]
    );
    console.log('     ✓ read');
    
    // sender_name (string)
    await databases.createStringAttribute(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      'sender_name',
      200,
      true,
      undefined,
      false,
      [Permission.read(Role.users()), Permission.create(Role.users())]
    );
    console.log('     ✓ sender_name');
    
    // Create indexes
    console.log('  → Creating indexes...');
    
    await databases.createIndex(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      'conversation_id_idx',
      'key',
      ['conversation_id'],
      ['ASC']
    );
    console.log('     ✓ conversation_id_idx');
    
    await databases.createIndex(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      'created_at_idx',
      'key',
      ['$createdAt'],
      ['ASC']
    );
    console.log('     ✓ created_at_idx');
    
    return collection;
  } catch (error) {
    if (error.code === 409) {
      console.log('ℹ️  Messages collection already exists, skipping...');
      return null;
    }
    throw error;
  }
}

// Main setup function
async function setup() {
  console.log('🚀 WarmStreet Messaging - Appwrite Database Setup\n');
  console.log('Endpoint:', APPWRITE_ENDPOINT);
  console.log('Project:', APPWRITE_PROJECT_ID);
  console.log('');
  
  try {
    // Create database
    await createDatabase();
    
    // Create collections
    await createConversationsCollection();
    await createMessagesCollection();
    
    console.log('\n✅ Setup complete!');
    console.log('\n📝 Summary:');
    console.log('  Database: ' + DATABASE_ID);
    console.log('  Collections:');
    console.log('    - ' + CONVERSATIONS_COLLECTION_ID + ' (conversations)');
    console.log('    - ' + MESSAGES_COLLECTION_ID + ' (messages)');
    console.log('\n🎉 Your messaging backend is ready!');
    console.log('\nNext steps:');
    console.log('1. Test messaging in your app');
    console.log('2. Monitor database usage in Appwrite console');
    console.log('3. Set up indexes for better query performance (already done!)');
    
  } catch (error) {
    console.error('\n❌ Setup failed:', error.message);
    if (error.response) {
      console.error('Response:', error.response);
    }
    process.exit(1);
  }
}

// Run setup
setup();
