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

#include "textfile.h"
#include "fiftyone.h"

static char* returnNextLine(
	char* buffer, 
	char* end, 
	char* current, 
	void* state, 
	void(*callback)(const char*, void*)) {

	while (current < end && *current != '\r' && *current != '\n') {
		current++;
	}

	// If there is an end of line character change it to a null and
	// call the callback.
	if (current < end) {
		*current = '\0';
		callback(buffer, state);
		// Move to the next character
		current++;
	}

	// Move to the next printable character.
	while (current < end && (*current == '\r' || *current == '\n')) {
		current++;
	}
	return current;
}

void fiftyoneDegreesTextFileIterateWithLimit(
	const char *fileName,
	char *buffer,
	int length,
	int limit,
	void *state,
	void(*callback)(const char*, void *)) {
	char* end = buffer + length;
	char* current = buffer;
	size_t bufferRead = 0;
	int counter = 0;
	FILE *handle;
	if (FileOpen(fileName, &handle) == SUCCESS) {
		while ((limit < 0 || counter < limit) &&
			(bufferRead = fread(current, sizeof(char), end - current, handle))
			== (size_t)(end - current)) {

			// Return the next line.
			current = returnNextLine(buffer, end, buffer, state, callback);
			counter++;

			// Shift the buffer to the left and load the next characters.
			size_t shift = end - current;
			memmove(buffer, current, shift);
			current = buffer + shift;
		}
		// Update end to the last line read
		end = current + bufferRead;
		if ((limit < 0 || counter < limit) && 
			(*(end - 1) != '\r' && *(end - 1) != '\n')) {
			// If there isn't a new line or carriage return at the end
			// we won't be able to determine the end of last line, so
			// set the end byte to '\n' and increase the end by 1.
			// This is safe as the buffer read at this point is always
			// smaller than the allocated size.
			*end = '\n';
			end++;
		}
		fclose(handle);

		// Return any final lines held in the buffer.
		while (current < end && 
			(limit < 0 || counter < limit)) {
			current = returnNextLine(buffer, end, buffer, state, callback);
			buffer = current;
			counter++;
		}
	}
}


void fiftyoneDegreesTextFileIterate(
	const char *fileName,
	char *buffer,
	int length,
	void *state,
	void(*callback)(const char*, void *)) {
	fiftyoneDegreesTextFileIterateWithLimit(
		fileName,
		buffer,
		length,
		-1,
		state,
		callback);
}
