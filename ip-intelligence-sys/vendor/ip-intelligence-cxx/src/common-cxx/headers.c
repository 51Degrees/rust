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

#include "headers.h"
#include "fiftyone.h"

/* HTTP header prefix used when processing collections of parameters. */
#define HTTP_PREFIX_UPPER "HTTP_"

/**
 * True if the value is not null, not zero length
 * and contains at least something meaningful besides pseudoheader separator characters
 */
static bool isHeaderValid(const char* value) {
    bool valid = false;
    
    while (value && *value != '\0' && !valid) {
        valid = *value != PSEUDO_HEADER_SEP;
        value++;
    }
    return valid;
}

/**
 * True if the value does not exist in the headers collection already, 
 * otherwise false.
 */
static bool isUnique(Headers* headers, const char* value) {
	for (uint32_t i = 0; i < headers->count; i++) {
		if (StringCompare(headers->items[i].name, value) == 0) {
			return false;
		}
	}
	return true;
}

/**
 * Free the members of the header.
 */
static void freeHeader(Header* header) {
	if (header->name != NULL) {
		Free((void*)header->name);
	}
	if (header->pseudoHeaders != NULL) {
		Free((void*)header->pseudoHeaders);
	}
	if (header->segmentHeaders != NULL) {
		Free((void*)header->segmentHeaders);
	}
}

/**
 * Sets all the header elements to default settings.
 */
static void initHeaders(Headers* headers) {
	for (uint32_t i = 0; i < headers->capacity; i++) {
		Header* h = &headers->items[i];
		h->index = i;
		h->headerId = 0;
		h->isDataSet = false;
		h->nameLength = 0;
		h->name = NULL;
		h->pseudoHeaders = NULL;
		h->segmentHeaders = NULL;
	}
}

/**
 * Counts the number of segments in a header name. 
 */
static int countHeaderSegments(const char* name) {
	int count = 0;
	char* current = (char*)name;
	char* last = current;

	// Loop checking each character ensuring at least some non separator 
	// characters are present before counting a segment.
	while (*current != '\0') {
		if (*current == PSEUDO_HEADER_SEP &&
			*last != PSEUDO_HEADER_SEP) {
			count++;
		}
		last = current;
		current++;
	}

	// If the last character was not a separator then the null terminator at 
	// the of the string indicates that there is a segment of valid characters
	// so increase the count.
	if (*last != PSEUDO_HEADER_SEP) {
		count++;
	}
	return count;
}

/**
 * Count the number of segments for all the headers.
 */
static int countAllSegments(void* state, HeadersGetMethod get) {
	uint32_t count = 0, index = 0, segments;
	const char* name;
	Item item;
	DataReset(&item.data);
	while (get(state, index, &item) >= 0) {
		name = STRING(item.data.ptr); // header is string
		if (isHeaderValid(name)) {

			// Count the number of segments.
			segments = countHeaderSegments(STRING(item.data.ptr)); // header is string
			count += segments;

			// If there are more than one segment then this is a pseudo header 
			// and the count should also include the full header.
			if (segments > 1) {
				count++;
			}
		}
		COLLECTION_RELEASE(item.collection, &item);
		index++;
	}
	return count;
}

/**
 * Relates the pseudoHeader index to the source.
 */
static void relateSegmentHeaderToPseudoHeader(
	Header* source, 
	Header* pseudoHeader) {
	HeaderPtrs* array = source->pseudoHeaders;
	array->items[array->count++] = pseudoHeader;
}

/**
 * Relates the pseudoHeader index to the source.
 */
static void relatePseudoHeaderToSegmentHeader(
	Header* pseudoHeader,
	Header* segmentHeader) {
	HeaderPtrs* array = pseudoHeader->segmentHeaders;
	array->items[array->count++] = segmentHeader;
}

/**
 * Copies the length of the source string characters to a new string array
 * associated with the header provided.
 */
static bool setHeaderName(
	Header* header, 
	const char* source, 
	size_t length, 
	Exception* exception) {
	size_t size = length + 1;
	char* name = (char*)Malloc(sizeof(char) * size);
	if (name == NULL) {
		EXCEPTION_SET(INSUFFICIENT_MEMORY);
		return false;
	}
	header->name = memcpy(name, source, length);
	if (header->name == NULL) {
		Free(name);
		return false;
	}
	name[length] = '\0';
	header->nameLength = length;
	return true;
}

/**
 * Sets the header with the name, and pseudo and related headers with the 
 * capacity provided.
 */
static bool setHeader(
	Header* header,
	const char* name,
	size_t length,
	uint32_t capacity,
	Exception* exception) {
	if (setHeaderName(header, name, length, exception) == false) {
		return false;
	}
	FIFTYONE_DEGREES_ARRAY_CREATE(
		fiftyoneDegreesHeaderPtr,
		header->pseudoHeaders,
		capacity);
	if (header->pseudoHeaders == NULL) {
		EXCEPTION_SET(INSUFFICIENT_MEMORY);
		freeHeader(header);
		return false;
	}
	FIFTYONE_DEGREES_ARRAY_CREATE(
		fiftyoneDegreesHeaderPtr,
		header->segmentHeaders,
		capacity);
	if (header->segmentHeaders == NULL) {
		EXCEPTION_SET(INSUFFICIENT_MEMORY);
		freeHeader(header);
		return false;
	}
	return true;
}

/**
 * Sets the header from the data set copying the data set string to new memory
 * specifically assigned for the header. Sets the capacity of the pseudo 
 * headers that might contain this header to the value provided.
 */
static bool setHeaderFromDataSet(
	Header* header,
	const char* name,
	size_t length,
    HeaderID headerId,
	uint32_t capacity,
	Exception* exception) {
	if (setHeader(header, name, length, capacity, exception) == false) {
		return false;
	}
	header->isDataSet = true;
	header->headerId = headerId;
	return true;
}

/**
 * Returns a pointer to the header if it exits, or NULL if it doesn't.
 */
static Header* getHeader(Headers* headers, const char* name, size_t length) {
	Header* item;
	for (uint32_t i = 0; i < headers->count; i++) {
		item = &headers->items[i];
		if (item->nameLength == length &&
			StringCompareLength(name, item->name, length) == 0) {
			return item;
		}
	}
	return NULL;
}

/**
 * Adds headers returned from the state and get method. The capacity of the
 * headers must be sufficient for the data set headers that will be returned.
 */
static bool addHeadersFromDataSet(
	void* state,
	HeadersGetMethod get,
	Headers* headers,
	Exception* exception) {
	Item item;
    long headerId;
	const char* name;
	uint32_t index = 0;
	DataReset(&item.data);

	// Loop through all the available headers in the data set adding those that
	// are valid and unique to the headers collection.
	while ((headerId = get(state, index++, &item)) >= 0) {
		name = STRING(item.data.ptr); // header is string
		if (isHeaderValid(name) && isUnique(headers, name)) {

			// Set the next header from the data set name item aborting if 
			// there was a problem.
			if (setHeaderFromDataSet(
				&headers->items[headers->count],
				name,
				strlen(name),
				(HeaderID)headerId,
				headers->capacity,
				exception) == false) {
				return false;
			}

			// Move to the next available header to be populated.
			headers->count++;
		}

		// Release the header name item before moving to the next one.
		COLLECTION_RELEASE(item.collection, &item);
	}
	return true;
}

/**
 * If a header with the provided name does not exist then add a new one to the
 * array of headers. Returns the header.
 */
static Header* getOrAddHeader(
	Headers* headers,
	const char* name,
	size_t length,
	Exception* exception) {
	Header* header = getHeader(headers, name, length);
	if (header == NULL) {
		header = &headers->items[headers->count];
		if (setHeader(
			header, 
			name, 
			length, 
			headers->capacity, 
			exception) == false) {
			return NULL;
		}
		headers->count++;
	}
	return header;
}

/**
 * Copies the source header into a new header in the headers array returning 
 * the copied header.
 */
static Header* copyHeader(
	Header* source, 
	Headers* headers, 
	Exception* exception) {
	Header* copied = &headers->items[headers->count++];
	copied->headerId = source->headerId;
	copied->isDataSet = source->isDataSet;
	copied->index = source->index;
	if (setHeaderName(
		copied, 
		source->name, 
		source->nameLength, 
		exception) == false) {
		return NULL;
	}
	FIFTYONE_DEGREES_ARRAY_CREATE(
		fiftyoneDegreesHeaderPtr,
		copied->pseudoHeaders,
		source->pseudoHeaders->count);
	if (copied->pseudoHeaders == NULL) {
		EXCEPTION_SET(INSUFFICIENT_MEMORY);
		freeHeader(copied);
		return NULL;
	}
	FIFTYONE_DEGREES_ARRAY_CREATE(
		fiftyoneDegreesHeaderPtr,
		copied->segmentHeaders,
		source->segmentHeaders->count);
	if (copied->segmentHeaders == NULL) {
		EXCEPTION_SET(INSUFFICIENT_MEMORY);
		freeHeader(copied);
		return NULL;
	}
	return copied;
}

/**
 * Gets or adds a header from the array and then creates the two way 
 * relationship between the pseudo header and the segment header.
 */
static bool addHeadersFromHeaderSegment(
	Headers* headers,
	Header* pseudoHeader,
	const char* name,
	size_t length,
	Exception* exception) {
	Header* segmentHeader = getOrAddHeader(
		headers,
		name,
		length,
		exception);
	if (segmentHeader == NULL) {
		return false;
	}

	// Relate the segment header to the pseudo header.
	relateSegmentHeaderToPseudoHeader(segmentHeader, pseudoHeader);

	// Relate the pseudo header to the segment header.
	relatePseudoHeaderToSegmentHeader(pseudoHeader, segmentHeader);

	return true;
}

/**
 * Extracts segments of pseudo headers ensuring they also exist in the headers 
 * array.
 */
static bool addHeadersFromHeader(
	Headers* headers, 
	Header* pseudoHeader, 
	Exception* exception) {
	uint32_t start = 0;
	uint32_t end = 0;
    bool separatorEncountered = false;
	for (;end < pseudoHeader->nameLength; end++) {

		// If a header has been found then either get the existing header with
		// this name, or add a new header.
		if (pseudoHeader->name[end] == PSEUDO_HEADER_SEP) {
            separatorEncountered = true;
            if (end - start > 0) {
                if (addHeadersFromHeaderSegment(
                                                headers,
                                                pseudoHeader,
                                                pseudoHeader->name + start,
                                                end - start,
                                                exception) == false) {
                                                    return false;
                                                }
            }

			// Move to the next segment.
			start = end + 1;
		}
	}

	// If there is a final segment then process this, but only if it is a pseudoheader
    // (i.e. separator was encountered) - do not do this for ordinary headers
	if (separatorEncountered && end - start > 0) {
		if (addHeadersFromHeaderSegment(
			headers,
			pseudoHeader,
			pseudoHeader->name + start,
			end - start,
			exception) == false) {
			return false;
		}
	}

	return true;
}

static bool addHeadersFromHeaders(Headers* headers, Exception* exception) {
    // cache count here, for it may change if headers are added
    // and thus loop may process additional headers which were not intended
    uint32_t count = headers->count;

	for (uint32_t i = 0; i < count; i++) {
		if (addHeadersFromHeader(
			headers, 
			&headers->items[i], 
			exception) == false) {
			return false;
		}
	}
	return true;
}

/**
 * Maintains the relationship between the source and destination headers using
 * instances for the related header from the headers array.
 */
static void copyRelationship(
	HeaderPtrs* src, 
	HeaderPtrs* dst, 
	Headers* headers) {
	assert(src->count == dst->capacity);
	for (uint32_t i = 0; i < src->count; i++) {
		dst->items[dst->count++] = &headers->items[src->items[i]->index];
	}
	assert(src->count == dst->count);
}

/**
 * Copies the relationship instances from the source header to the copied 
 * instance in the destination.
 */
static void copyRelationships(Header* src, Header* dst, Headers* headers) {
	copyRelationship(src->pseudoHeaders, dst->pseudoHeaders, headers);
	copyRelationship(src->segmentHeaders, dst->segmentHeaders, headers);
}

/**
 * The arrays are initial created with more capacity than they are likely to 
 * need. To reduce the amount of data held in memory beyond the creation of the
 * headers to a minimum and thus enable the most efficient operation a copy of 
 * the headers structure is created using only the memory required.
 */
static Headers* trimHeaders(Headers* source, Exception* exception) {
	Headers* trimmed;
	FIFTYONE_DEGREES_ARRAY_CREATE(
		fiftyoneDegreesHeader,
		trimmed,
		source->count);
	if (trimmed != NULL) {

		// Initialise all the headers.
		initHeaders(trimmed);

		// Copy the headers, but not the relationship between segments and 
		// pseudos. This is done once all instances are created in the trimmed
		// array.
		for (uint32_t i = 0; i < source->count; i++) {
			if (copyHeader(&source->items[i], trimmed, exception) == NULL) {
				HeadersFree(trimmed);
				return NULL;
			}
		}

		// Copy the relationships to the trimmed header instances.
		for (uint32_t i = 0; i < source->count; i++) {
			copyRelationships(&source->items[i], &trimmed->items[i], trimmed);
		}

		// Finally free the sources now a trimmed copies has been made.
		HeadersFree(source);
	}
	return trimmed;
}

fiftyoneDegreesHeaders* fiftyoneDegreesHeadersCreate(
	bool expectUpperPrefixedHeaders,
	void *state,
	fiftyoneDegreesHeadersGetMethod get,
	fiftyoneDegreesException* exception) {
	Headers* headers;

	// Count the number of headers and create an array with sufficient capacity
	// to store all of them.
	int32_t count = countAllSegments(state, get);
	FIFTYONE_DEGREES_ARRAY_CREATE(
		fiftyoneDegreesHeader, 
		headers, 
		count);
	if (headers != NULL) {

		// Initialise all the headers.
		initHeaders(headers);

		// Add the headers from the data set.
		if (addHeadersFromDataSet(state, get, headers, exception) == false) {
			HeadersFree(headers);
			return NULL;
		}

		// Add headers from the headers already added where there are pseudo
		// headers present.
		if (addHeadersFromHeaders(headers, exception) == false) {
			HeadersFree(headers);
			return NULL;
		}

		// Trim the capacity of all the array to reduce operational memory.
		headers = trimHeaders(headers, exception);
		if (headers == NULL) {
			return NULL;
		}

		// Set the prefixed headers flag.
		headers->expectUpperPrefixedHeaders = expectUpperPrefixedHeaders;
	}
	return headers;
}

int fiftyoneDegreesHeaderGetIndex(
	fiftyoneDegreesHeaders *headers,
	const char* httpHeaderName,
	size_t length) {
	uint32_t i;
	Header* header;

	// Check if header is from a Perl or PHP wrapper in the form of HTTP_*
	// and if present skip these characters.
	if (headers->expectUpperPrefixedHeaders == true &&
		length > sizeof(HTTP_PREFIX_UPPER) &&
		StringCompareLength(
			httpHeaderName,
			HTTP_PREFIX_UPPER,
			sizeof(HTTP_PREFIX_UPPER) - 1) == 0) {
		length -= sizeof(HTTP_PREFIX_UPPER) - 1;
		httpHeaderName += sizeof(HTTP_PREFIX_UPPER) - 1;
	}

	// Perform a case insensitive compare of the remaining characters.
	for (i = 0; i < headers->count; i++) {
		header = &headers->items[i];
		if (header->nameLength == length &&
			StringCompareLength(
				httpHeaderName, 
				header->name,
				length) == 0) {
			return i;
		}
	}

	return -1;
}

fiftyoneDegreesHeader* fiftyoneDegreesHeadersGetHeaderFromUniqueId(
	fiftyoneDegreesHeaders *headers,
	HeaderID uniqueId) {
	uint32_t i;
	for (i = 0; i < headers->count; i++) {
		if (headers->items[i].headerId == uniqueId) {
			return &headers->items[i];
		}
	}
	return (Header*)NULL;
}

void fiftyoneDegreesHeadersFree(fiftyoneDegreesHeaders *headers) {
	uint32_t i;
	if (headers != NULL) {
		for (i = 0; i < headers->count; i++) {
			freeHeader(&headers->items[i]);
		}
		Free((void*)headers);
		headers = NULL;
	}
}

bool fiftyoneDegreesHeadersIsHttp(
	void *state,
	const char* field,
	size_t length) {
	return HeaderGetIndex((Headers*)state, field, length) >= 0;
}
