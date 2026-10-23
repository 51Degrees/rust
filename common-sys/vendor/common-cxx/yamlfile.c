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

#include "yamlfile.h"
#include "fiftyone.h"

// Number of sequential characters needed to indicate a control block.
static const size_t CONTROL_LENGTH = 3;

typedef struct pair_state_t {
	KeyValuePair* pairs; // array of key value evidence pairs
	uint16_t size; // total available keyValuePairs
	int16_t index; // current index being populated in the keyValuePairs
	char* current; // current pointer to character in the pair
	char* end; // last pointer that is valid
} PairState;

typedef struct file_state_t {
	FILE* handle; // handle to the yaml file
	char* buffer; // start of the buffer
	const size_t bufferLength; // length of the buffer
	char* end; // last character in the buffer that is valid
	char* current; // current character in the buffer
	size_t dashCount; // number of continuous dashes read
	size_t dotCount; // number of continuous dots read
	size_t newLineCount; // new line characters read
	size_t quote; // position of the first quote in the value
	size_t position; // character index on the current line
	size_t colon; // position of the colon in the current line
} FileState;

static StatusCode readNextBlock(FileState* fileState) {
	StatusCode status = SUCCESS;
	size_t bytesRead = fread(
		fileState->buffer,
		sizeof(char),
		fileState->bufferLength,
		fileState->handle);
	if (bytesRead == 0 && !feof(fileState->handle)) {
		status = INSUFFICIENT_MEMORY;
	}
	else if (ferror(fileState->handle)) {
		status = FILE_READ_ERROR;
	}
	else if (bytesRead == 0) {
		status = FILE_END_OF_FILE;
	}
	fileState->end = fileState->buffer + bytesRead - 1;
	fileState->current = fileState->buffer;
	return status;
}

// Read the next character or null if there are no more characters to be read.
static char* readNext(FileState* fileState) {
	StatusCode status = SUCCESS;
	fileState->current++;
	if (fileState->current > fileState->end) {
		status = readNextBlock(fileState);
		if (status != SUCCESS) {
			return NULL;
		}
	}
	return fileState->current;
}

// Sets the current and end pointers to the current key.
static void setCurrentKey(PairState* state) {
	KeyValuePair* current = state->pairs + state->index;
	state->current = (char*)current->key;
	state->end = (char*)(current->key + current->keyLength - 1);
}

// Sets the current and end pointers to the current value.
static void setCurrentValue(PairState* state) {
	KeyValuePair* current = state->pairs + state->index;
	state->current = (char*)current->value;
	state->end = (char*)current->value + current->valueLength - 1;
}

// Switches from writing to the current key to the current value. Ensures that
// the current string being written is null terminated.
static void switchKeyValue(PairState* state) {
	*state->current = '\0';
	setCurrentValue(state);
}

// Advances to the next key value pair. Ensures that the current string being 
// written is null terminated.
static void nextPair(PairState* state) {
	*state->current = '\0';
	state->index++;
	setCurrentKey(state);
}

// Sets the counters for a new line.
static void newLine(FileState* state) {
	state->newLineCount = 0;
	state->dotCount = 0;
	state->dashCount = 0;
	state->position = 0;
	state->colon = 0;
	state->quote = 0;
}

// Sets the pairs for a new document.
static void newDocument(PairState* state) {
	state->index = -1;
}

// Move the character position along by one.
static void advance(FileState* state) {
	state->newLineCount = 0;
	state->position++;
}

// True if the character from the file is a value and not a control character.
static bool isValue(FileState* state) {
	return state->colon == 0 || state->position > state->colon + 1;
}

// Adds the character to the key value pair if the conditions are met.
static void addCharacter(
	PairState* pairState,
	FileState* fileState,
	char* current) {
	if (pairState->current < pairState->end &&
		pairState->index < pairState->size &&
		isValue(fileState)) {
		*pairState->current = *current;
		pairState->current++;
	}
}

// Removes wrapping double quotes that YAML serializers add around a value
// where the first non-whitespace character is a single quote 
// e.g. "'hello world" -> 'hello world.
// Client Hint headers such as "Chromium";v="8" whose double quotes
// are part of the value are left untouched.
static void stripDoubleQuoteWrap(PairState* state) {
	char* start = (char*)(state->pairs + state->index)->value;
	// The last character written to the value. current points one beyond it.
	char* last = state->current - 1;
	if (last > start && *start == '"' && *last == '"') {
		char* inner = start + 1;
		while (inner < last && (*inner == ' ' || *inner == '\t')) {
			inner++;
		}
		if (inner < last && *inner == '\'') {
			// Shift the body left over the opening quote and drop the trailing
			// quote by moving the write pointer back over both quotes.
			memmove(start, start + 1, (size_t)(last - (start + 1)));
			state->current -= 2;
		}
	}
}

StatusCode fiftyoneDegreesYamlFileIterateWithLimit(
	const char* fileName,
	char* buffer,
	size_t length,
	KeyValuePair* keyValuePairs,
	uint16_t collectionSize,
	int limit,
	void* state,
	void(*callback)(KeyValuePair*, uint16_t, void*)) {
	char* current;
	FILE *handle;
	int count = 0;
	StatusCode status = FileOpen(fileName, &handle);
	if (status != SUCCESS) {
		return status;
	}

	FileState fileState = { 
		handle, 
		buffer, 
		length,
		// Set the end and current to 0 to force the next block to be read.
		0,
		0,
		0, 
		0,
		0,
		0,
		0,
		false };

	PairState pairState = {
		keyValuePairs,
		collectionSize,
		0,
		(char*)keyValuePairs[0].key,
		(char*)(keyValuePairs[0].key + keyValuePairs[0].keyLength - 1) };

	// If there is no limit then set the limit to the largest value to 
	// avoid checking for negative values in the loop.
	if (limit < 0) {
		limit = INT_MAX;
	}

	while (true) {
		current = readNext(&fileState);

		// If EOF or the first new line then move to the next pair.
		if (!current || *current == '\n' || *current == '\r') {
			// Only finalise a pair if one is actually in progress. Position is
			// zero on a blank line or an empty file so finalising there would 
			// emit a phantom empty.
			if (fileState.newLineCount == 0 && fileState.position > 0) {

				// If the value opened with a single quote and the last
				// character is also a single quote then remove the trailing
				// quote (the opening one was skipped while reading).
				if (fileState.quote > 0 &&
					*(pairState.current - 1) == '\'') {
					pairState.current--;
				}
				// If there is a value, remove any wrapping double
				// quotes the serializer added around a leading single quote.
				else if (fileState.colon > 0) {
					stripDoubleQuoteWrap(&pairState);
				}
				nextPair(&pairState);
			}
			if (current) {
				newLine(&fileState);
				fileState.newLineCount++;
			}
			else {
				// EOF
				if (pairState.index > 0) {
					callback(keyValuePairs, pairState.index, state);
				}
				break;
			}
		}

		// If a dash and control length is reached then return the pairs
		// and reset ready for the next set.
		else if (*current == '-' && fileState.position == fileState.dashCount) {
			advance(&fileState);
			fileState.dashCount++;
			if (fileState.dashCount == CONTROL_LENGTH) {
				if (pairState.index > 0)
				{
					callback(keyValuePairs, pairState.index, state);
					count++;
					if (count >= limit) {
						break;
					}
				}
				newDocument(&pairState);
			}
		}

		// If a dot and control length then return and exit.
		else if (*current == '.' && fileState.position == fileState.dotCount) {
			advance(&fileState);
			fileState.dotCount++;
			if (fileState.dotCount == CONTROL_LENGTH) {
				if (pairState.index > 0)
				{
					callback(keyValuePairs, pairState.index, state);
				}
				break;
			}
		}

		// If the first colon then switch adding from the key to the value.
		// Record the colon having been passed.
		else if (*current == ':' && fileState.colon == 0) {
			advance(&fileState);
			switchKeyValue(&pairState);
			fileState.colon = fileState.position;
		}

		// If this is the first quote after the colon and the space then don't
		// add the character to the value and record the fact a colon has been
		// found. This will be used at the end of the line to remove the last
		// quote if one is present. Other quotes within the string will be 
		// treated normally.
		else if (*current == '\'' && 
			fileState.colon > 0 && 
			fileState.colon + 1 == fileState.position) {
			advance(&fileState);
			fileState.quote = fileState.position;
		}

		// Not a control character just add to the current key or value if
		// there is space remaining.
		else {
			advance(&fileState);
			addCharacter(&pairState, &fileState, current);
		}
	}
	fclose(handle);
	
	return status;
}

StatusCode fiftyoneDegreesYamlFileIterate(
	const char* fileName,
	char* buffer,
	size_t length,
	KeyValuePair* keyValuePairs,
	uint16_t collectionSize,
	void* state,
	void(*callback)(KeyValuePair*, uint16_t, void*)) {
	return fiftyoneDegreesYamlFileIterateWithLimit(
		fileName,
		buffer,
		length,
		keyValuePairs,
		collectionSize,
		-1,
		state,
		callback);
}
