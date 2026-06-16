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

#include "ip.h"
#include "fiftyone.h"

typedef void(*parseIterator)(
	void *state,
	IpType segmentType,
	const char *start,
	const char *end);

/**
 * State is an integer which is increased every time the method is called.
 */
static void callbackIpAddressCount(
	void * const state,
	const IpType segmentType,
	const char * const start,
	const char * const end) {
	if (start <= end) {
		if (segmentType != IP_TYPE_INVALID) {
			(*(int*)state)++;
			if (segmentType == IP_TYPE_IPV6) {
				(*(int*)state)++;
			}
		}
	}
}

/*
 * Make sure each byte in the Ipv4 or Ipv6 address
 * stays within the bound 0,255
 * @parsedValue The value parsed from string
 * @return the adjusted value
 * if the value is out of the range then return
 * the closest boundary value (0 or 255)
 */
static byte getIpByte(int parsedValue) {
	if (parsedValue < 0) {
		parsedValue = 0;
	}
	else if (parsedValue > UINT8_MAX) {
		parsedValue = UINT8_MAX;
	}
	return (byte)parsedValue;
}

typedef struct {
	IpAddress * const address;
	byte *current;
	int bytesPresent;
	int abbreviationsFilled;
} fiftyoneDegreeIpAddressBuildState;
typedef fiftyoneDegreeIpAddressBuildState IpAddressBuildState;

static void parseIpV6Segment(
	IpAddressBuildState * const buildState,
	const char * const start,
	const char * const end) {
	int i;
	char first[3], second[3], val; // "FF" + '\0'
	if (start > end) {
		// This is an abbreviation, so fill it in.
		if (buildState->abbreviationsFilled) {
			return;
		}
		buildState->abbreviationsFilled++;
		for (i = 0; i < IPV6_LENGTH - buildState->bytesPresent; i++) {
			*buildState->current = (byte)0;
			buildState->current++;
		}
	}
	else {
		// Add the two bytes of the segment.
		first[2] = '\0';
		second[2] = '\0';
		for (i = 0; i < IPV4_LENGTH; i++) {
			if (end - i >= start) val = end[-i];
			else val = '0';

			if (i < 2) second[1 - i] = val;
			else first[3 - i] = val;
		}
		*buildState->current = getIpByte((int)strtol(first, NULL, 16));
		buildState->current++;
		*buildState->current = getIpByte((int)strtol(second, NULL, 16));
		buildState->current++;
	}
}

static void callbackIpAddressBuild(
	void * const state,
	const IpType segmentType,
	const char * const start,
	const char * const end) {
	fiftyoneDegreeIpAddressBuildState *const buildState = state;
	if (segmentType == IP_TYPE_IPV4) {
		*buildState->current = getIpByte(atoi(start));
		buildState->current++;
	}
	else if (segmentType == IP_TYPE_IPV6) {
		parseIpV6Segment(buildState, start, end);
	}
}

static IpType getIpTypeFromSeparator(const char separator) {
	switch (separator) {
		case '.':
			return IP_TYPE_IPV4;
		case ':':
			return IP_TYPE_IPV6;
		default:
			return IP_TYPE_INVALID;
	}
}

static IpType getSegmentTypeWithSeparator(
	const char separator,
	const IpType ipType,
	const IpType lastSeparatorType) {
	switch (ipType) {
		case IP_TYPE_IPV4:
			return IP_TYPE_IPV4;
		case IP_TYPE_IPV6:
			switch (separator) {
				case ':':
					return IP_TYPE_IPV6;
				case '.':
					return IP_TYPE_IPV4;
				default:
					return lastSeparatorType;
			}
		default:
			return getIpTypeFromSeparator(separator);
	}
}

typedef enum {
	FIFTYONE_DEGREES_IP_NON_BREAK_CHAR = 0,
	FIFTYONE_DEGREES_IP_SEGMENT_BREAK_CHAR,
	FIFTYONE_DEGREES_IP_ADDRESS_BREAK_CHAR,
	FIFTYONE_DEGREES_IP_INVALID_CHAR,
} IpStringSeparatorType;

#define IP_NON_BREAK_CHAR FIFTYONE_DEGREES_IP_NON_BREAK_CHAR
#define IP_SEGMENT_BREAK_CHAR FIFTYONE_DEGREES_IP_SEGMENT_BREAK_CHAR
#define IP_ADDRESS_BREAK_CHAR FIFTYONE_DEGREES_IP_ADDRESS_BREAK_CHAR
#define IP_INVALID_CHAR FIFTYONE_DEGREES_IP_INVALID_CHAR

static IpStringSeparatorType getSeparatorCharType(
	const char ipChar,
	const IpType ipType) {

	switch (ipChar) {
		case ':':
			return ((ipType == IP_TYPE_IPV4)
				? IP_ADDRESS_BREAK_CHAR : IP_SEGMENT_BREAK_CHAR);
		case '.':
			return IP_SEGMENT_BREAK_CHAR;
		case ',':
		case ' ':
		case ']':
		case '/':
		case '\0':
		case '\n':
			return IP_ADDRESS_BREAK_CHAR;
		default:
			break;
	}
	if ('0' <= ipChar && ipChar <= '9') {
		return IP_NON_BREAK_CHAR;
	}
	if (('a' <= ipChar && ipChar <= 'f') || ('A' <= ipChar && ipChar <= 'F')) {
		return (ipType == IP_TYPE_IPV4
			? IP_INVALID_CHAR : IP_NON_BREAK_CHAR);
	}
	return IP_INVALID_CHAR;
}

static int8_t getMaxSegmentLengthForIpType(const IpType ipType) {
	return (ipType == IP_TYPE_IPV4) ? 3 : 4; // "255" or "FFFF"
}

/**
 * Calls the callback method every time a byte is identified in the value
 * when parsed left to right.
 */
static IpType iterateIpAddress(
	const char *start,
	const char * const end,
	void * const state,
	int * const springCount,
	IpType type,
	const parseIterator foundSegment) {

	const char * const postEnd = end + 1;

	*springCount = 0;
	if (*start == '[') {
		if (type == IP_TYPE_IPV4) {
			return IP_TYPE_INVALID;
		}
		start++;
	}

	IpType currentSegmentType =
		IP_TYPE_INVALID;

	const char *current = start;
	const char *nextSegment = current;
	for (; current <= postEnd && nextSegment <= postEnd; ++current) {
		char nextChar = 0;
		if (current < postEnd) {
			nextChar = *current;
		}
		IpStringSeparatorType separatorType =
			getSeparatorCharType(nextChar, type);
		if (!separatorType) {
			continue;
		}
		if (separatorType == IP_INVALID_CHAR) {
			return IP_TYPE_INVALID;
		}

		currentSegmentType = getSegmentTypeWithSeparator(
			nextChar, type, currentSegmentType);
		if (type == IP_TYPE_INVALID) {
			type = currentSegmentType;
		}

		if (current - nextSegment > getMaxSegmentLengthForIpType(type)) {
			return IP_TYPE_INVALID;
		}

		// Check if it is leading abbreviation
		if (current - 1 >= start) {
			foundSegment(state, currentSegmentType,
				nextSegment, current - 1);
		}
		if (separatorType == IP_ADDRESS_BREAK_CHAR) {
			return type;
		}
		if (current == nextSegment && current != start) {
			++*springCount;
		}
		nextSegment = current + 1;
	}
	if (nextSegment < current
		&& type != IP_TYPE_INVALID) {

		foundSegment(state, currentSegmentType,
			nextSegment, current - 1);
	}
	return type;
}

bool fiftyoneDegreesIpAddressParse(
	const char * const start,
	const char * const end,
	IpAddress * const address) {

	if (!start) {
		return false;
	}

	int byteCount = 0;
	int springCount = 0;
	IpType type = iterateIpAddress(
		start,
		end,
		&byteCount,
		&springCount,
		IP_TYPE_INVALID,
		callbackIpAddressCount);

	switch (type) {
		case IP_TYPE_IPV4:
			if (byteCount != IPV4_LENGTH || springCount) {
				return false;
			}
			break;
		case IP_TYPE_IPV6:
			if (byteCount > IPV6_LENGTH ||
				springCount > 1 ||
				(byteCount < IPV6_LENGTH && !springCount)) {
				return false;
			}
			break;
		default:
			return false;
	}

	address->type = type;
	IpAddressBuildState buildState = {
		address,
		address->value,
		byteCount,
		0,
	};

	// Add the bytes from the source value and get the type of address.
	iterateIpAddress(
		start,
		end,
		&buildState,
		&springCount,
		type,
		callbackIpAddressBuild);

	return true;
}

int fiftyoneDegreesIpAddressesCompare(
	const unsigned char * const ipAddress1,
	const unsigned char * const ipAddress2,
	IpType type) {
	uint16_t compareSize = 0;
	int result = 0;
	switch(type) {
	case IP_TYPE_IPV4:
		compareSize = IPV4_LENGTH;
		break;
	case IP_TYPE_IPV6:
		compareSize = IPV6_LENGTH;
		break;
	case IP_TYPE_INVALID:
	default:
		compareSize = 0;
		break;
	}

	for (uint16_t i = 0; i < compareSize; i++) {
		result = ipAddress1[i] - ipAddress2[i];
		if (result != 0) return result;
	}
	return result;
}
