import { User } from '../models/user';

export function login(username: string, password: string): User | null {
  return null;
}

export class AuthService {
  authenticate(user: User): boolean {
    return true;
  }
}
