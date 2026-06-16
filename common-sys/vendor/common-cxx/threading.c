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

#include "threading.h"
#include "fiftyone.h"

#ifdef _MSC_VER
Signal* SignalCreate()  {
	Signal *signal = (Signal*)CreateEventEx(
		NULL,
		NULL,
		CREATE_EVENT_INITIAL_SET,
		EVENT_MODIFY_STATE | SYNCHRONIZE);
	assert(signal != NULL);
	return signal;
}
void SignalClose(Signal *signal) {
	if (signal != NULL) {
		CloseHandle(signal);
	}
}

#ifdef _MSC_VER
#pragma warning (push)
#pragma warning (disable: 4189) 
#endif
void SignalSet(Signal *signal) {
	BOOL result = SetEvent(signal);
	assert(result != 0);
}
#ifdef _MSC_VER
#pragma warning (pop)
#endif

#ifdef _MSC_VER
#pragma warning (push)
#pragma warning (disable: 4189) 
#endif
void SignalWait(Signal *signal) {
	DWORD result = WaitForSingleObject(signal, INFINITE);
	assert(result == WAIT_OBJECT_0);
}
#ifdef _MSC_VER
#pragma warning (pop)
#endif

#else

#ifdef __APPLE__
#include <sys/time.h>
#endif

/**
 * GCC / PTHREAD SIGNAL IMPLEMENTATION - NOT USED BY WINDOWS
 */

void fiftyoneDegreesMutexCreate(fiftyoneDegreesMutex *mutex) {
#ifdef _DEBUG
	int result =
#endif
	pthread_mutex_init(mutex, NULL);
	assert(result == 0);
}

void fiftyoneDegreesMutexClose(fiftyoneDegreesMutex *mutex) {
	pthread_mutex_destroy(mutex);
}

void fiftyoneDegreesMutexLock(fiftyoneDegreesMutex *mutex) {
	pthread_mutex_lock(mutex);
}

void fiftyoneDegreesMutexUnlock(fiftyoneDegreesMutex *mutex) {
	pthread_mutex_unlock(mutex);
}

fiftyoneDegreesSignal* fiftyoneDegreesSignalCreate() {
	Signal *signal = (Signal*)Malloc(sizeof(Signal));
	if (signal != NULL) {
		signal->wait = false;
		if (pthread_cond_init(&signal->cond, NULL) != 0 ||
			pthread_mutex_init(&signal->mutex, NULL) != 0) {
			Free(signal);
			signal = NULL;
		}
	}
	return signal;
}

void fiftyoneDegreesSignalClose(fiftyoneDegreesSignal *signal) {
	if (signal != NULL) {
		pthread_mutex_destroy(&signal->mutex);
		pthread_cond_destroy(&signal->cond);
	}
}

void fiftyoneDegreesSignalSet(fiftyoneDegreesSignal *signal) {
	if (pthread_mutex_lock(&signal->mutex) == 0) {
		signal->wait = false;
		pthread_cond_signal(&signal->cond);
		pthread_mutex_unlock(&signal->mutex);
	}
}

void fiftyoneDegreesSignalWait(fiftyoneDegreesSignal *signal) {
	if (pthread_mutex_lock(&signal->mutex) == 0) {
		while (signal->wait == true) {
#ifdef _DEBUG
			int result =
#endif
			pthread_cond_wait(&signal->cond, &signal->mutex);
			assert(result == 0);
		}
		signal->wait = true;
		pthread_mutex_unlock(&signal->mutex);
	}
}

#endif

bool fiftyoneDegreesThreadingGetIsThreadSafe() {
#if FIFTYONE_DEGREES_NO_THREADING
	return false;
#else
	return true;
#endif
}