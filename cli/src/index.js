import { Command } from 'commander';

export function createProgram() {
  const program = new Command();
  program
    .name('codegraph')
    .description('AST-based code graph generator')
    .version('0.1.0');

  return program;
}
