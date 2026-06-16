/* *********************************************************************
 * This Original Work is copyright of 51 Degrees Mobile Experts Limited.
 * Copyright 2026 51 Degrees Mobile Experts Limited, Davidson House,
 * Forbury Square, Reading, Berkshire, United Kingdom RG1 3EU.
 *
 * This Original Work is licensed under the European Union Public Licence
 * (EUPL) v.1.2 and is subject to its terms as set out below.
 *
 * If a copy of the EUPL was not distributed with this file, You can obtain
 * one at https://opensource.org/licenses/EUPL-1.2.
 *
 * The 'Compatible Licences' set out in the Appendix to the EUPL (as may be
 * amended by the European Commission) shall be deemed incompatible for
 * the purposes of the Work and the provisions of the compatibility
 * clause in Article 5 of the EUPL shall not apply.
 *
 * If using the Work as, or as part of, a network application, by
 * including the attribution notice(s) required under Article 5 of the EUPL
 * in the end user terms of the application under an appropriate heading,
 * such notice(s) shall fulfill the requirements of that article.
 * ********************************************************************* */

#include "evidence.h"
#include "fiftyone.h"

typedef struct evidence_iterate_state_t {
	EvidenceKeyValuePairArray *evidence;
	EvidencePrefix prefix;
	void *state;
	EvidenceIterateMethod callback;
} evidenceIterateState;

typedef struct evidence_find_state_t {
	Header* header; // Header to find
	EvidenceKeyValuePair* pair; // Pair found that matches the header
} evidenceFindState;

static EvidencePrefixMap _map[] = {
	{ "server.", sizeof("server.") - 1, 
	FIFTYONE_DEGREES_EVIDENCE_SERVER },
	{ "header.", sizeof("header.") - 1, 
	FIFTYONE_DEGREES_EVIDENCE_HTTP_HEADER_STRING },
	{ "query.", sizeof("query.") - 1, FIFTYONE_DEGREES_EVIDENCE_QUERY },
	{ "cookie.", sizeof("cookie.") - 1, FIFTYONE_DEGREES_EVIDENCE_COOKIE }
};

static void parsePair(EvidenceKeyValuePair *pair) {
	switch (pair->prefix) {
	case FIFTYONE_DEGREES_EVIDENCE_HTTP_HEADER_IP_ADDRESSES:
	case FIFTYONE_DEGREES_EVIDENCE_HTTP_HEADER_STRING:
	case FIFTYONE_DEGREES_EVIDENCE_SERVER:
	case FIFTYONE_DEGREES_EVIDENCE_QUERY:
	case FIFTYONE_DEGREES_EVIDENCE_COOKIE:
	default:
		// These are string prefixes so just copy over the original values.
		pair->parsedValue = pair->item.value;
		pair->parsedLength = pair->item.valueLength;
		break;
	}
}

// If a string comparison of the pair field and the header indicates a match
// then set the header to avoid a string comparison in future iterations.
static void setPairHeader(EvidenceKeyValuePair* pair, Header* header) {
	if (pair->item.keyLength == header->nameLength &&
		StringCompareLength(
			pair->item.key, 
			header->name, 
			header->nameLength) == 0) {
		pair->header = header;
	}
}

/**
 * Iterate through an evidence collection and perform callback on the evidence
 * whose prefix matches the input prefixes. Checks the linked list of evidence
 * arrays to ensure these are also processed.
 *
 * @param evidence the evidence collection to process
 * @param prefixes the accepted evidence prefixes
 * @param state the state object to hold the current state of the process
 * @param callback the method to call back when a matched evidence is found.
 * @return number of evidence processed.
 */
static uint32_t evidenceIterate(
	EvidenceKeyValuePairArray* evidence,
	int prefixes,
	void* state,
	EvidenceIterateMethod callback) {
	uint32_t index = 0, iterations = 0;
	EvidenceKeyValuePair* pair;
	bool cont = true;
	while (cont && evidence != NULL) {

		// Check the current evidence item and call back if the right prefix
		// after parsing the pair if not done so already.
        if (index < evidence->count) {
            pair = &evidence->items[index++];
            if ((pair->prefix & prefixes) == pair->prefix) {
                if (pair->parsedValue == NULL) {
                    parsePair(pair);
                }
                cont = callback(state, pair);
                iterations++;
            }
        }

		// Check if the next evidence array needs to be moved to.
		if (index >= evidence->count) {
			evidence = evidence->next;
			index = 0;
		}
	}
	return iterations;
}

/**
 * If the header name and pair key match then stop iterating having set the 
 * found pair, otherwise return false.
 */
static bool findHeaderEvidenceCallback(
	void* state,
	EvidenceKeyValuePair* pair) {
	evidenceFindState* findState = (evidenceFindState*)state;
	if (findState->header == pair->header || (
		findState->header->nameLength == pair->item.keyLength &&
		StringCompareLength(
			findState->header->name,
			pair->item.key,
			pair->item.keyLength) == 0)) {
		findState->pair = pair;
		return false;
	}
	return true;
}

/**
 * Finds the evidence pair that matches the header. Returns null if a pair does
 * not exist.
 */
static EvidenceKeyValuePair* findHeaderEvidence(
	EvidenceKeyValuePairArray* evidence,
	int prefixes,
	Header* header) {
	evidenceFindState state = { header, NULL };
	evidenceIterate(evidence, prefixes, &state, findHeaderEvidenceCallback);
	return state.pair;
}

// Safe-copies the pair parsed value to the buffer checking that there are
// sufficient bytes remaining in the buffer for the parsed value.
static void addPairValueToBuffer(
	StringBuilder* builder, 
	EvidenceKeyValuePair* pair) {
	StringBuilderAddChars(
		builder, 
		(char*)pair->parsedValue, 
		pair->parsedLength);
}

// For the header finds the corresponding evidence in the array of evidence. If
// found then copies the parsed value into the buffer considering the remaining
// length available. Returns true if successful

static bool addHeaderValueToBuilder(
	fiftyoneDegreesEvidenceKeyValuePairArray* evidence,
	int prefixes,
	fiftyoneDegreesHeader* header,
	StringBuilder* builder,
    bool prependSeparator) {

	// Get the evidence that corresponds to the header. If it doesn't exist
	// then there is no evidence for the header and a call back will not be
	// possible.
	EvidenceKeyValuePair* pair = findHeaderEvidence(
		evidence, 
		prefixes, 
		header);
	if (pair == NULL) {
		return false;
	}

    // Add the pseudo header separator.
    if (prependSeparator) {
        StringBuilderAddChar(builder, PSEUDO_HEADER_SEP);
    }

	// Copy the value of the evidence pair in to the buffer advancing the
	// current character in the buffer.
	addPairValueToBuffer(builder, pair);
    
    // Return false if we have overfilled the buffer.
    return builder->full == false;
}

// Assembles a pseudo header in the buffer. If this can not be achieved returns 
// true to indicate that processing should continue. If a pseudo header can be
// created then returns the result of the callback which might decide not to 
// continue processing.
static bool processPseudoHeader(
	EvidenceKeyValuePairArray* evidence,
	int prefixes,
	Header* header,
	StringBuilder* builder,
	void* state,
	fiftyoneDegreesEvidenceIterateMethod callback) {
	EvidenceKeyValuePair pair;

	// For each of the headers that form the pseudo header.
	for (uint32_t i = 0; i < header->segmentHeaders->count; i++) {
        
		// if this is a subsequent segment - we prepend the separator
        bool prependSeparator = i > 0;

		// Add the header evidence that forms the segment if available updating
		// the current buffer position if available.
		bool success = addHeaderValueToBuilder(
			evidence, 
			prefixes, 
			header->segmentHeaders->items[i], 
			builder, 
			prependSeparator);

		// If the pseudo header wasn't found, or insufficient space was 
		// available to copy it, then return.
		if (!success) {
			return true;  // which means continue iteration
		}
	}

	// Append (or overwrite if it is the last character) a null terminating 
	// character.
	StringBuilderComplete(builder);

	// A full header has been formed so call the callback with an evidence pair 
	// containing the parsed value.
	pair.item.key = NULL;
	pair.item.keyLength = 0;
	pair.header = header;
	pair.item.value = builder->ptr;
	// As we've added the null terminator, the "added" is 1 longer than
	// the actual string length.
	pair.item.valueLength = builder->added - 1;
	pair.parsedValue = builder->ptr;
	pair.parsedLength = builder->added - 1;
	pair.prefix = 0;
	return callback(state, &pair);
}

// Finds the header in the evidence, and if available calls the callback. 
// Returns true if further processing should continue, otherwise false to stop
// further processing.
static bool processHeader(
	EvidenceKeyValuePairArray* evidence,
	int prefixes,
	Header* header,
	void* state,
	fiftyoneDegreesEvidenceIterateMethod callback) {

	// Get the evidence that corresponds to the header. If it doesn't exist
	// then there is no evidence for the header and a call back will not be
	// possible.
	EvidenceKeyValuePair* pair = findHeaderEvidence(
		evidence,
		prefixes,
		header);
	if (pair == NULL) {
		return true;
	}

	// A full header has been formed so call the callback with the pair.
	return callback(state, pair);
}

fiftyoneDegreesEvidenceKeyValuePairArray*
fiftyoneDegreesEvidenceCreate(uint32_t capacity) {
	fiftyoneDegreesEvidenceKeyValuePairArray *evidence;
	uint32_t i;
	FIFTYONE_DEGREES_ARRAY_CREATE(EvidenceKeyValuePair, evidence, capacity);
	if (evidence != NULL) {
		evidence->next = NULL;
		evidence->prev = NULL;
		for (i = 0; i < evidence->capacity; i++) {
			evidence->items[i].item.key = NULL;
			evidence->items[i].item.keyLength = 0;
			evidence->items[i].item.value = NULL;
			evidence->items[i].item.valueLength = 0;
			evidence->items[i].header = NULL;
			evidence->items[i].parsedValue = NULL;
			evidence->items[i].parsedLength = 0;
			evidence->items[i].prefix = FIFTYONE_DEGREES_EVIDENCE_IGNORE;
		}
	}
	return evidence;
}

void fiftyoneDegreesEvidenceFree(
	fiftyoneDegreesEvidenceKeyValuePairArray *evidence) {
    if (evidence == NULL) {
        return;
    }
	EvidenceKeyValuePairArray* current = evidence;
	while (current->next != NULL) {
		current = current->next;
	}
	while (current != NULL) {
		evidence = current->prev;
		Free(current);
		current = evidence;
	}
}

fiftyoneDegreesEvidenceKeyValuePair* fiftyoneDegreesEvidenceAddPair(
	fiftyoneDegreesEvidenceKeyValuePairArray *evidence,
	fiftyoneDegreesEvidencePrefix prefix,
	fiftyoneDegreesKeyValuePair value) {
	EvidenceKeyValuePair *pair = NULL;
	while (pair == NULL) {
		if (evidence->count < evidence->capacity) {
			// Use the next item in the allocated array.
			pair = &evidence->items[evidence->count++];
			pair->prefix = prefix;
			pair->item = value;
			pair->parsedValue = NULL;
			pair->header = NULL;
		}
		else {
			// If there is insufficient capacity in the evidence array then add
			// a new array.
			if (evidence->next == NULL) {
				evidence->next = EvidenceCreate(
					evidence->capacity == 0 ? 1 : evidence->capacity);
				evidence->next->prev = evidence;
			}
			// Move to the next evidence array.
			evidence = evidence->next;
		}
	}
	return pair;
}

fiftyoneDegreesEvidenceKeyValuePair* fiftyoneDegreesEvidenceAddString(
	fiftyoneDegreesEvidenceKeyValuePairArray* evidence,
	fiftyoneDegreesEvidencePrefix prefix,
	const char* key,
	const char* value) {
	KeyValuePair pair = { key, strlen(key), value, strlen(value) };
	return EvidenceAddPair(evidence, prefix, pair);
}

uint32_t fiftyoneDegreesEvidenceIterate(
	fiftyoneDegreesEvidenceKeyValuePairArray *evidence,
	int prefixes,
	void *state,
	fiftyoneDegreesEvidenceIterateMethod callback) {
	return evidenceIterate(
		evidence,
		prefixes,
		state,
		callback);
}

fiftyoneDegreesEvidencePrefixMap* fiftyoneDegreesEvidenceMapPrefix(
	const char *key) {
	uint32_t i;
	size_t length = strlen(key);
	EvidencePrefixMap *map;
    EvidencePrefixMap *result = NULL;
	for (i = 0; i < sizeof(_map) / sizeof(EvidencePrefixMap); i++) {
		map = &_map[i];
		if (map->prefixLength < length &&
			strncmp(map->prefix, key, map->prefixLength) == 0) {
			result = map;
            break;
		}
	}
	return result;
}

const char* fiftyoneDegreesEvidencePrefixString(
	fiftyoneDegreesEvidencePrefix prefix) {
	uint32_t i;
	EvidencePrefixMap* map;
    const char *result = NULL;
	for (i = 0; i < sizeof(_map) / sizeof(EvidencePrefixMap); i++) {
		map = &_map[i];
		if (map->prefixEnum == prefix) {
            result = map->prefix;
            break;
		}
	}
	return result   ;
}

bool fiftyoneDegreesEvidenceIterateForHeaders(
	fiftyoneDegreesEvidenceKeyValuePairArray* evidence,
	int prefixes,
	fiftyoneDegreesHeaderPtrs* headers,
	char* const buffer,
	size_t const length,
	void* state,
	fiftyoneDegreesEvidenceIterateMethod callback) {
	Header* header;
	StringBuilder builder = { buffer, length };

	// For each of the headers process as either a standard header, or a pseudo
	// header.
	for (uint32_t i = 0; i < headers->count; i++) {
		header = headers->items[i];

		// Try and process the header as a standard header.
		if (processHeader(
			evidence,
			prefixes,
			header,
			state,
			callback) == false) {
			return true;
		}

		// If the header is a pseudo header then attempt to assemble a complete
		// value from the evidence and process it. Note: if there is only one
		// segment then that will be the header that was already processed in 
		// processHeader therefore there is no point processing the same value
		// a second time as a pseudo header.
		if (buffer != NULL && 
			header->segmentHeaders != NULL &&
			header->segmentHeaders->count > 1) {
			StringBuilderInit(&builder);
			if (processPseudoHeader(
				evidence,
				prefixes,
				header,
				&builder,
				state,
				callback) == false) {
				return true;
			}
		}
	}

	return false;
}
