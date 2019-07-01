//  Copyright 2019 Twitter, Inc
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

#include <float.h>
#include <stdint.h>
#include <stdio.h>

#include "counter.h"
#include "histogram.h"

int main() {
    printf("Counter:\n");
    counter_t *counter = counter_new();
    printf("count is: %d\n", counter_count(counter));
    counter_add(counter, 1);
    printf("count is: %d\n", counter_count(counter));
    counter_sub(counter, 1);
    printf("count is: %d\n", counter_count(counter));
    counter_sub(counter, 1);
    printf("count is: %d\n", counter_count(counter));
    counter_clear(counter);
    printf("count is: %d\n", counter_count(counter));
    printf("\n");

    printf("Histogram:\n");
    histogram_t *histogram = histogram_new(1, 60000000000, 3);
    printf("total entries: %d\n", histogram_samples(histogram));
    for(uintptr_t i = 1; i <= 100; i++) {
        histogram_incr(histogram, i, 1);
    }
    printf("total entries: %d\n", histogram_samples(histogram));
    printf("pMin: %d\n", histogram_percentile(histogram, 0.0));
    printf("p50: %d\n", histogram_percentile(histogram, 0.5));
    printf("p75: %d\n", histogram_percentile(histogram, 0.75));
    printf("p90: %d\n", histogram_percentile(histogram, 0.9));
    printf("p99: %d\n", histogram_percentile(histogram, 0.99));
    printf("p999: %d\n", histogram_percentile(histogram, 0.999));
    printf("pMax: %d\n", histogram_percentile(histogram, 1.0));
    return 0;
}
