# Bugfix Requirements Document

## Introduction

The WhatsApp chatbot does not respond to messages in two related scenarios: (1) when the user messages themselves (same phone number as the bot) for testing purposes, and (2) when the chatbot feature is enabled but the sender's phone number is not registered as a tenant. Both issues stem from the backend's sender policy enforcement in the incoming webhook handler, which silently discards messages from phone numbers not found in the `inquilino` table — including the bot's own number during self-messaging tests.

## Bug Analysis

### Current Behavior (Defect)

1.1 WHEN a user sends a WhatsApp message to themselves (same phone number as the bot) AND the chatbot is enabled (`activo = true`) AND the sender policy is `tenants_only` THEN the system silently discards the message because the bot's own phone number is not registered as a tenant in the `inquilino` table

1.2 WHEN a user sends a WhatsApp message to themselves (same phone number as the bot) AND the chatbot is enabled (`activo = true`) AND the sender policy is `allowlist` AND the bot's phone number is not in the allowlist THEN the system silently discards the message without generating a response

1.3 WHEN the chatbot feature is enabled (`activo = true`) with default configuration (`sender_policy = "tenants_only"`) AND a message arrives from a phone number that is the connected WhatsApp session's own number THEN the system returns `{"status": "discarded"}` with no AI processing or reply sent

### Expected Behavior (Correct)

2.1 WHEN a user sends a WhatsApp message to themselves (same phone number as the bot) AND the chatbot is enabled (`activo = true`) THEN the system SHALL bypass the sender policy check and process the message through the full AI pipeline regardless of the configured sender policy

2.2 WHEN a user sends a WhatsApp message to themselves (same phone number as the bot) AND the chatbot is enabled (`activo = true`) THEN the system SHALL generate an AI response and send it back via the Baileys service to the same phone number

2.3 WHEN a self-message is detected (sender phone matches the connected WhatsApp session's phone number) AND the chatbot is enabled THEN the system SHALL treat the message as a valid test interaction and persist both the user message and assistant reply in the conversation history

### Unchanged Behavior (Regression Prevention)

3.1 WHEN a message arrives from a phone number that is NOT the bot's own number AND the sender policy is `tenants_only` AND the phone is not registered as a tenant THEN the system SHALL CONTINUE TO silently discard the message

3.2 WHEN a message arrives from a phone number that is NOT the bot's own number AND the sender policy is `allowlist` AND the phone is not in the allowlist THEN the system SHALL CONTINUE TO silently discard the message

3.3 WHEN the chatbot is disabled (`activo = false`) THEN the system SHALL CONTINUE TO silently discard all incoming messages regardless of sender phone

3.4 WHEN a message arrives from a phone number that is NOT the bot's own number AND the sender policy is `tenants_and_prospects` THEN the system SHALL CONTINUE TO allow the message through for AI processing

3.5 WHEN a message arrives from a registered tenant's phone number AND the sender policy is `tenants_only` THEN the system SHALL CONTINUE TO allow the message through for AI processing

3.6 WHEN the baileys-service receives a message that was sent programmatically by the bot (echoed bot reply) THEN the system SHALL CONTINUE TO filter it out via the `sentMessageIds` tracking set and NOT forward it to the backend
