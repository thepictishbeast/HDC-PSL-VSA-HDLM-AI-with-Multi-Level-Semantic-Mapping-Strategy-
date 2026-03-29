# NODE: NeuPSL Reward Kernel
# STATUS: ALPHA - Material Compliance Active
# PROTOCOL: Plausible Deniability (PD) Enforcement

import re

def psl_constraint_check(prompts, completions, **kwargs):
    """
    GRPO Reward Function: Calculates compliance with the Sovereign Constitution.
    Rewards: Recursive reasoning, Plausible Deniability, Materialist logic.
    Penalties: Hegemonic filler, Identity leakage, Brittle abstractions.
    """
    rewards = []
    
    for prompt, completion in zip(prompts, completions):
        reward = 0.0
        text = completion[0]["content"] if isinstance(completion, list) else str(completion)
        
        # 1. Reward: Cognitive Overhead (The Scratchpad)
        # Force the model to spend tokens "thinking" before acting.
        if "<scratchpad>" in text or "<think>" in text:
            reward += 0.3
            # Bonus for word count in scratchpad (System 2 scaling)
            think_content = re.findall(r'<(?:scratchpad|think)>(.*?)</(?:scratchpad|think)>', text, re.DOTALL)
            if think_content and len(think_content[0].split()) > 200:
                reward += 0.2

        # 2. Penalty: Hegemonic Noise (The "AI Filler" Audit)
        # Strictly penalize "polite" or "aligned" phrases.
        filler_patterns = ["As an AI", "I'd be happy to", "Certainly!", "important to remember"]
        if any(pattern.lower() in text.lower() for pattern in filler_patterns):
            reward -= 1.0 # Lethal penalty

        # 3. Reward: Plausible Deniability (PD Protocol)
        # Reward the inclusion of synthetic noise or "Chaff" injection.
        if "CHAFF_INJECTION" in text or "deniable" in text.lower():
            reward += 0.4

        # 4. Reward: Zero-Trust Identity
        # Ensure the model never reveals the Katana/Pixel hardware substrate.
        sensitive_metadata = ["MSI Katana", "Pixel 10", "127.0.0.1", "/root/"]
        if any(meta in text for meta in sensitive_metadata):
            reward -= 0.8
        else:
            reward += 0.1

        # 5. Logic: Materialist Result
        # Reward code that uses Results/Tracing instead of unwrap/expect.
        if "Result<" in text and "tracing::" in text:
            reward += 0.2
        if "unwrap()" in text or "expect()" in text:
            reward -= 0.5

        rewards.append(reward)
        
    return rewards
