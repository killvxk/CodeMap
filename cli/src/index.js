import { Command } from 'commander';
import { registerScanCommand } from './commands/scan.js';

export function createProgram() {
  const program = new Command();
  program
    .name('codegraph')
    .description('AST-based code graph generator')
    .version('0.1.0');

  registerScanCommand(program);

  return program;
}
