#!/usr/bin/env tsx
/**
 * Test smart truncation behavior
 * Quick validation without MCP restart
 */

// Test cases from user's burp
const testCases = [
  {
    name: "Short message (should show full)",
    content: "ctx::2025-10-23 @ 04:22 PM - Issue #580 VERIFIED WORKING\n\nUser tested and confirmed: \"woot, works!\"\n\nImplementation now functioning correctly.",
    expected: "full"
  },
  {
    name: "Medium message with good sentence boundary",
    content: "ctx::2025-10-23 @ 04:18 PM - Found and fixed Issue #580 implementation problem\n\nproject::pharmacy-online/rangle\nissue::580\n\n## Root Cause:\nThe working directory version of apps/web/components/assessment-flow had stale cache preventing the new auto-add-to-basket logic from executing. Fix applied by clearing build cache and verifying AssessmentComplete.tsx triggers addToBasket correctly. All tests passing.",
    expected: "sentence boundary after 'correctly'"
  },
  {
    name: "Long message (should truncate cleanly)",
    content: "ctx::2025-10-23 @ 04:17 PM - Issue #580 implementation not working as expected\n\nproject::pharmacy-online/rangle\nissue::580\n\n**Expected**: Complete assessment → auto-add to basket → redirect to checkout\n**Actual**: Assessment completes but nothing happens, user stuck on success page\n\n## Investigation:\n1. Checked AssessmentComplete component - addToBasket call present\n2. Verified basket store reducer - logic looks correct\n3. Console logs show basket state not updating\n4. Suspect stale build or cache issue\n\n## Next Steps:\n- Clear build cache\n- Verify component re-compilation\n- Test with fresh dev server",
    expected: "truncate with clean break"
  }
];

// Simple truncation logic (mirrors the implementation)
function smartTruncate(content: string, maxLength: number = 400): string {
  if (content.length <= maxLength) {
    return content;
  }

  const searchEnd = Math.min(maxLength + 50, content.length);
  const searchText = content.substring(0, searchEnd);
  const sentenceEndings = [...searchText.matchAll(/[.!?]\s+/g)];

  if (sentenceEndings.length > 0) {
    const lastEnding = sentenceEndings[sentenceEndings.length - 1];
    const endPos = (lastEnding.index || 0) + lastEnding[0].length - 1;

    if (endPos > maxLength - 100) {
      return content.substring(0, endPos).trim();
    }
  }

  const wordBoundary = content.lastIndexOf(' ', maxLength);
  if (wordBoundary > maxLength - 50) {
    return content.substring(0, wordBoundary).trim() + '...';
  }

  return content.substring(0, maxLength).trim() + '...';
}

console.log('🧪 Testing Smart Truncation\n');

testCases.forEach((test, idx) => {
  console.log(`\n--- Test ${idx + 1}: ${test.name} ---`);
  console.log(`Original length: ${test.content.length} chars`);

  const truncated = smartTruncate(test.content);
  console.log(`Truncated length: ${truncated.length} chars`);
  console.log(`Expected behavior: ${test.expected}`);

  console.log('\nResult:');
  console.log(truncated);
  console.log('\n' + '='.repeat(80));
});

console.log('\n✅ Smart truncation test complete!\n');
console.log('Compare with old 200-char hard truncation:');
testCases.forEach((test, idx) => {
  const oldTruncate = test.content.substring(0, 200) + (test.content.length > 200 ? '...' : '');
  console.log(`\nTest ${idx + 1} (old): ${oldTruncate.substring(0, 100)}...`);
});
