import { getUserById } from '../db/users';
import bcrypt from 'bcrypt';

export interface LoginOptions {
  email: string;
  password: string;
}

export async function login(opts: LoginOptions): Promise<string> {
  const user = await getUserById(opts.email);
  const valid = await bcrypt.compare(opts.password, user.hash);
  if (!valid) throw new Error('Invalid credentials');
  return 'token';
}
