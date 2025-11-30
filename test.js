// Simple test file for meeting detection

const { isMeetingActive, onMeetingStart, onMeetingEnd, init, getLastDetectionDetails } = require('./index');

console.log('🧪 Testing Meeting Detection Engine\n');

// Initialize
console.log('Initializing engine...');
init();

// Test current status
console.log('\n📊 Current Status:');
const active = isMeetingActive();
console.log(`Meeting active: ${active ? '✅ YES' : '❌ NO'}`);

// Register event handlers
console.log('\n🎧 Registering event handlers...');

onMeetingStart(() => {
  console.log('\n---------------------------------🎥 [EVENT] Meeting started!---------------------------------');
  console.log('   Timestamp:', new Date().toISOString());
});

onMeetingEnd(() => {
  console.log('\n---------------------------------✅ [EVENT] Meeting ended!---------------------------------');
  console.log('   Timestamp:', new Date().toISOString());
});

// Keep process alive
process.on('SIGINT', () => {
  console.log('\n\n👋 Shutting down...');
  process.exit(0);
});

