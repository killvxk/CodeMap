import { Command } from 'commander';
import { registerScanCommand } from './commands/scan.js';
import { registerQueryCommand } from './commands/query.js';
import { registerUpdateCommand } from './commands/update.js';
import { registerImpactCommand } from './commands/impact.js';
import { registerStatusCommand } from './commands/status.js';
import { registerSliceCommand } from './commands/slice.js';

export function createProgram() {
  const program = new Command();
  program
    .name('codegraph')
    .description('AST-based code graph generator')
    .version('0.1.0');

  registerScanCommand(program);
  registerQueryCommand(program);
  registerUpdateCommand(program);
  registerImpactCommand(program);
  registerStatusCommand(program);
  registerSliceCommand(program);

  return program;
}
