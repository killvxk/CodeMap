import { login } from '../auth/login';

export function handleLogin(req: any, res: any) {
  const token = login(req.body);
  res.json({ token });
}
