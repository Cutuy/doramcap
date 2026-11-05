import { createContext, useContext } from 'react';
import { api, type createClient } from './api';

export type Client = ReturnType<typeof createClient>;
/** What the screens share: the client they call and the hash prefix their links live under. */
export const ConsoleContext = createContext<{ client: Client; basePath: string }>({ client: api, basePath: '' });
export const useConsole = () => useContext(ConsoleContext);
