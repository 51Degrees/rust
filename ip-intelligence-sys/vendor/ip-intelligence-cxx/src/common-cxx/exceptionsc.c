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

#include "exceptions.h"
#include "fiftyone.h"

#ifndef FIFTYONE_DEGREES_EXCEPTIONS_DISABLED

const char* fiftyoneDegreesExceptionGetMessage(
	fiftyoneDegreesException *exception) {
	const char format[] =
		"Message:  %s\r\n"
		"Source:   %s\r\n"
		"Function: %s\r\n"
		"Line:     %d\r\n";
	size_t length;
	char *exceptionMessage = NULL;
	const char *statusMessage = StatusGetMessage(
		exception->status,
		NULL);
	if (statusMessage != NULL) {
		length = strlen(format);
		length += strlen(statusMessage);
		length += exception->file != NULL ? strlen(exception->file) : 0;
		length += exception->func != NULL ? strlen(exception->func) : 0;
		length += 10; // add 10 for line number
		exceptionMessage = (char*)Malloc(length);
		Snprintf(
			exceptionMessage,
			length,
			format,
			statusMessage,
			exception->file,
			exception->func,
			exception->line);
		Free((void*)statusMessage);
	}
	return exceptionMessage;
}

void fiftyoneDegreesExceptionCheckAndExit(
	fiftyoneDegreesException *exception) {
	if (EXCEPTION_OKAY == false) {
		const char *message = ExceptionGetMessage(
			exception); \
			if (message != NULL) {
				fputs(message, stderr);
				fiftyoneDegreesFree((void*)message);
			}
		exit(exception->status);
	}
}

#else

#ifdef _MSC_VER
#pragma warning (push)
#pragma warning (disable: 4100) 
#endif

const char* fiftyoneDegreesExceptionGetMessage(
	fiftyoneDegreesException *exception) {
	return "";
}
void fiftyoneDegreesExceptionCheckAndExit(
	fiftyoneDegreesException *exception) {
}

#ifdef _MSC_VER
#pragma warning (default: 4100) 
#pragma warning (pop)
#endif

#endif