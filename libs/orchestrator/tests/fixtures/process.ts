// Standalone child process for protocol, error, capture-limit and timeout tests.
switch (process.argv[2]) {
  case 'protocol': {
    process.stdin.setEncoding('utf8');
    let input = '';
    for await (const chunk of process.stdin) input += chunk;
    const request = JSON.parse(input);
    console.log(JSON.stringify(request.input.value + 3));
    console.error(request.protocol);
    break;
  }
  case 'failure':
    console.error('bad');
    process.exitCode = 7;
    break;
  case 'invalid':
    console.log('invalid');
    break;
  case 'overflow':
    console.log('x'.repeat(100000));
    break;
  case 'timeout':
    setTimeout(() => {}, 30000);
    break;
  default:
    throw new Error('unknown process fixture mode');
}
