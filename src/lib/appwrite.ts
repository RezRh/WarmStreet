import { Client, Account, Databases, Storage } from 'appwrite';

const client = new Client()
    .setEndpoint('https://sgp.cloud.appwrite.io/v1')
    .setProject('69b47d6d002eda226e97');

export const account = new Account(client);
export const databases = new Databases(client);
export const storage = new Storage(client);

export default client;
