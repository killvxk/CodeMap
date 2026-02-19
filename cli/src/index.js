import { Command } from 'commander';
import { registerScanCommand } from './commands/scan.js';
import { registerQueryCommand } from './commands/query.js';
import { registerUpdateCommand } from './commands/update.js';

export function createProgram() {
  const program = new Command();
  program
    .name('codegraph')
    .description('AST-based code graph generator')
    .version('0.1.0');

  registerScanCommand(program);
  registerQueryCommand(program);
  registerUpdateCommand(program);

  return program;
}
