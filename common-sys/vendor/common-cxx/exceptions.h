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

#ifndef FIFTYONE_DEGREES_EXCEPTIONS_H_INCLUDED
#define FIFTYONE_DEGREES_EXCEPTIONS_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesExceptions Exceptions
 *
 * Allow the bubbling up or errors in C.
 *
 * ## Introduction
 *
 * An exception structure is used to allow the bubbling up of errors, as C
 * does not support exceptions in that way. This means that instead of an error
 * causing a segmentation fault elsewhere, the exception is set and passed all
 * the way up to be checked.
 *
 * ## Creating
 * 
 * Exceptions are created by the caller using the
 * #FIFTYONE_DEGREES_EXCEPTION_CREATE macro which creates an exception pointer
 * named "exception". This is then passed into any method which can potentially
 * throw an exception.
 *
 * ## Checking
 *
 * An exception can be checked using the #FIFTYONE_DEGREES_EXCEPTION_OKAY macro
 * which will return true if there is no exception, or false if an exception
 * has occurred.
 *
 * ## Throwing
 *
 * In the event that an exception has occurred in a method, it can be checked
 * and thrown using the #FIFTYONE_DEGREES_EXCEPTION_THROW macro. If the
 * exception is okay, then nothing will be thrown by this macro, so it is safe
 * to call as a "catch and throw" method. This will behave differently
 * depending on whether it is used in the context of C or C++.
 *
 * **C** : C does not support exceptions, so if there is an exception, the
 * exception message will be printed to standard output, then the process will
 * exit.
 *
 * **C++** : As C++ supports exceptions, a fatal exception with the message
 * will be thrown. This can then be caught or handled in whichever way the
 * caller sees fit.
 *
 * ## Messages
 *
 * The error message returned by an exception consists of the error message
 * itself, the name of the source file which caused the error, the name of the
 * function which caused the error and the line in the source file at which the
 * error occurred.
 *
 * ## Usage Example
 *
 * ```
 * // Create an exception
 * FIFTYONE_DEGREES_EXCEPTION_CREATE;
 * // Set the exception a failure status
 * FIFTYONE_DEGREES_EXCEPTION_SET(FIFTYONE_DEGREES_STATUS_NULL_POINTER);
 * // Check the exception
 * if  (FIFTYONE_DEGREES_EXCEPTION_FAILED) {
 *     // Throw the exception
 *     FIFTYONE_DEGREES_EXCEPTION_THROW;
 * }
 * ```
 *
 * ## Disabling
 *
 * To improve performance, exception handling can be disabled completely by
 * compiling with FIFTYONE_DEGREES_EXCEPTIONS_DISABLED. This changes all the
 * macro calls to do nothing, meaning that no checks occur, and no exceptions
 * are thrown.
 *
 * @{
 */

#include "status.h"
#include "common.h"

#ifndef FIFTYONE_DEGREES_EXCEPTIONS_DISABLED

/**
 * Structure used to represent a 51Degrees exception and passed into methods
 * that might generate exceptions. The #FIFTYONE_DEGREES_EXCEPTION_SET macro
 * should be used to set the status code.
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 */
EXTERNAL typedef struct fiftyone_degrees_exception_t {
	const char *file; /**< File generating the exception */
	const char *func; /**< Function generating the exception */
	int line; /**< Line number generating the exception */
	fiftyoneDegreesStatusCode status; /**< Status code to assign */
} fiftyoneDegreesException;

/**
 * Macro used to set an exception to a status code.
 * @param s status code to set
 */
#define FIFTYONE_DEGREES_EXCEPTION_SET(s) \
if (exception != NULL) { \
exception->file = __FILE__; \
exception->func = __func__; \
exception->line = __LINE__; \
exception->status = s; \
}

/**
 * Macro used to clear an exception type.
 */
#define FIFTYONE_DEGREES_EXCEPTION_CLEAR \
{ \
exception->file = NULL; \
exception->func = NULL; \
exception->line = -1; \
exception->status = FIFTYONE_DEGREES_STATUS_NOT_SET; \
}

/**
* Macro used to check if an exception status equals the value of t.
* Warning: this macro should be avoided in anything other than test code as 
* when exceptions are disabled there will be unpredictable results. Using a
* local status variable for checks is a better pattern.
*/
#define FIFTYONE_DEGREES_EXCEPTION_CHECK(t) \
(exception == NULL || exception->status == t)

/**
 * Macro used to check if there is no exception currently.
 */
#define FIFTYONE_DEGREES_EXCEPTION_OKAY \
FIFTYONE_DEGREES_EXCEPTION_CHECK(FIFTYONE_DEGREES_STATUS_NOT_SET)

#ifdef FIFTYONE_DEGREES_EXCEPTIONS_HPP

/**
 * Macro to throw a C++ exception if the C exception is set. Only used if C++
 * exceptions are enabled.
 */
#define FIFTYONE_DEGREES_EXCEPTION_THROW \
if (FIFTYONE_DEGREES_EXCEPTION_OKAY == false) { \
throw FiftyoneDegrees::Common::FatalException(exception); \
}

#else

/**
 * Macro to print to standard error a message if an exception is set.
 */
#define FIFTYONE_DEGREES_EXCEPTION_THROW \
fiftyoneDegreesExceptionCheckAndExit(exception);

#endif

#else

EXTERNAL typedef void* fiftyoneDegreesException;

#define FIFTYONE_DEGREES_EXCEPTION_CLEAR

#define FIFTYONE_DEGREES_EXCEPTION_SET(s)

#define FIFTYONE_DEGREES_EXCEPTION_OKAY true

#define FIFTYONE_DEGREES_EXCEPTION_THROW

#define FIFTYONE_DEGREES_EXCEPTION_CHECK(t) false

#endif

/**
 * Macro used to create an exception.
 */
#define FIFTYONE_DEGREES_EXCEPTION_CREATE \
fiftyoneDegreesException exceptionValue; \
fiftyoneDegreesException *exception = &exceptionValue; \
FIFTYONE_DEGREES_EXCEPTION_CLEAR

/**
 * Macro to test if the exception has been set and is failed.
 */
#define FIFTYONE_DEGREES_EXCEPTION_FAILED \
(FIFTYONE_DEGREES_EXCEPTION_OKAY == false)

/**
 * Returns an English error message for the exception. The caller must free the
 * memory when they have finished consuming the error message.
 * @param exception to get a string message from
 * @return pointer to the newly allocated message string
 */
EXTERNAL const char* fiftyoneDegreesExceptionGetMessage(
	fiftyoneDegreesException *exception);

/**
 * If the exception is set then will print a message to stderr and exit the 
 * process. 
 * @param exception to check and exit if set
 */
EXTERNAL void fiftyoneDegreesExceptionCheckAndExit(
	fiftyoneDegreesException *exception);

/**
 * @}
 */

#endif